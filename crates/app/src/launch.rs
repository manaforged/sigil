//! `sigil::launch` — the one-call entry point.
//!
//! Owns the full frame pipeline: reactive scope → shell/window → renderer
//! → lazy surface → cx/layout/router → event drain → scene emit → paint.
//! Every failure mode listed in `phase-08a-ergonomics-pass.md`'s
//! bulletproof table becomes unreachable at this layer.

use std::sync::Arc;

use sigil_events::EventRouter;
use sigil_hal::{Color, LogicalSize, PhysicalSize, Renderer as _, Scale, Shell};
use sigil_kernel::Cx;
use sigil_layout::LayoutEngine;
use sigil_layout::prelude::Size;
use sigil_reactive::{
    MainThreadDispatcher, Runtime, TaskExecutor, next_interval_deadline, tick_intervals,
};
use sigil_renderer::{WgpuRenderer, WgpuSurface, emit_scene};
use sigil_shell::{WinitShell, WinitWindowHandle};
use sigil_tasks::AsyncRuntime;

use crate::config::{AppConfig, PanicPolicy};
use crate::error::{AppResult, Error};
use crate::text_measure::DefaultTextMeasure;
use crate::tree_root::TreeRoot;

mod event_wiring;

use event_wiring::{DrainCtx, EnsureSurfaceCtx, drain_events, ensure_surface, sync_ime};

/// Launch a sigil app.
///
/// `config` accepts `()`, `&str`, `(title, w, h)`, or `AppConfig`.
/// `tree` accepts either `FnOnce(&mut Cx) -> NodeId` or a `NodeBuilder`.
///
/// Returns when the user closes the window or (by default) presses Esc.
///
/// # Errors
///
/// Returns an error if the shell, renderer, or layout engine fails to
/// initialise. Runtime errors during the frame loop (surface lost,
/// transient wgpu timeouts) are handled internally — the loop retries
/// on the next frame.
pub fn launch<C, T>(config: C, tree: T) -> AppResult<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    Runtime::scope("sigil-app", || run(config, tree))
}

fn run<T: TreeRoot>(config: AppConfig, tree: T) -> AppResult<()> {
    let logical_size = logical_size_from_config(config.size);

    let mut shell = if config.overlay {
        WinitShell::new_accessory()
    } else {
        WinitShell::new()
    }
    .map_err(Error::new)?;
    let window_handle = create_initial_window(&mut shell, &config, logical_size)?;

    let mut async_rt = install_async_runtime(&shell)?;

    let mut cx = Cx::new_root();
    let root = tree.install(&mut cx);

    let theme_watch = start_theme_watch(&config, &mut cx, root);

    let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
    // Install pointer→glyph geometry so text widgets can hit-test clicks and
    // draw caret/selection overlays (mirrors the measurer install above).
    sigil_layout::set_text_geometry(std::rc::Rc::new(crate::EngineGeometry::new()));
    layout_for_scale(&mut cx, root, &mut layout, Scale::ONE, logical_size).map_err(Error::new)?;

    let mut router = EventRouter::new();
    // W5-A11Y (DR-050 §1) — clone the inbound `ActionRequest` receiver
    // before `shell` is moved into `Shell::run` below; the frame loop
    // drains it every frame via `drain_a11y_actions`.
    let a11y_action_rx = shell.a11y_action_receiver();
    let mut renderer = WgpuRenderer::new()?;
    if config.overlay {
        renderer.set_transparent(true);
    }
    let mut surface: Option<WgpuSurface> = None;
    let mut viewport = (logical_size.width, logical_size.height);
    let mut exit_requested = false;

    // Wake the loop when the soonest `cx.interval` is due, so periodic
    // callbacks (clocks, live metrics) fire without waiting on an OS
    // input event. The shell turns this into `ControlFlow::WaitUntil`;
    // `None` (no intervals registered) falls back to `Wait` at zero cost.
    // Paired with the `tick_intervals` drain in `frame_once`.
    shell.on_idle_deadline(next_interval_deadline);

    let panic_policy = config.panic_policy;
    shell
        .run(move |shell| {
            let frame = FrameCtx {
                config: &config,
                shell,
                window_handle,
                cx: &mut cx,
                root,
                layout: &mut layout,
                router: &mut router,
                renderer: &mut renderer,
                surface: &mut surface,
                viewport: &mut viewport,
                exit_requested: &mut exit_requested,
                async_rt: &mut async_rt,
                a11y_action_rx: &a11y_action_rx,
                theme_watch: &theme_watch,
            };
            run_frame(panic_policy, frame);
        })
        .map_err(Error::new)?;
    Ok(())
}

/// G5 (DR-048 §5) — build the async runtime and install the Tokio
/// executor + reactive bridge so `Context::spawn` has somewhere to send
/// its futures. Called BEFORE the tree is installed, so trees may
/// `Context::spawn` during build — matching the fixture contract in
/// `cx_spawn.rs`. The bridge's worker→main mpsc is drained at the top of
/// every frame, just before `Runtime::begin_effect` consumes the reactive
/// `pending_completions` queue.
///
/// The bridge waker wakes the winit loop whenever a worker thread posts a
/// reactive completion, so background async work (e.g. streamed agent
/// output via `Context::spawn` + `AsyncContext::update`) repaints promptly
/// instead of stalling in `ControlFlow::Wait` until an unrelated OS input
/// event. The wake delivers a `WakeEvent` through the shell's
/// `EventLoopProxy`, whose `user_event` handler requests a redraw → next
/// `frame_once` drains the bridge via `pump_completions_into_reactive`.
/// (DR-048 §5 left this waker a no-op.)
fn install_async_runtime(shell: &WinitShell) -> AppResult<AsyncRuntime> {
    let mut async_rt = AsyncRuntime::new().map_err(Error::new)?;
    let waker = shell.waker();
    async_rt.set_bridge_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    install_g5_executor(&async_rt);
    Ok(async_rt)
}

/// G5 (DR-048 §5) — wire `Context::spawn` to the supplied async runtime.
///
/// Public-in-crate so the integration test
/// `crates/app/tests/g5_executor.rs` can install the same pair against
/// its own `AsyncRuntime` and drive the same frame-loop pump path as
/// production.
pub fn install_g5_executor(async_rt: &AsyncRuntime) {
    let executor: Arc<dyn TaskExecutor> = Arc::new(async_rt.tokio_executor());
    let dispatcher: Arc<dyn MainThreadDispatcher> = Arc::new(async_rt.reactive_bridge());
    Runtime::install_executor(executor, dispatcher);
}

/// T14 (5/5 Wave 4) — start the dev-mode theme watch and attach the
/// file's current theme. Parse failure falls back to the default theme:
/// the edit-see loop must never kill the app for a bad save.
fn start_theme_watch(
    config: &AppConfig,
    cx: &mut Cx,
    root: sigil_kernel::NodeId,
) -> Option<crate::theme_file::ThemeWatch> {
    let watch = crate::theme_file::start(config.theme_file.as_deref())?;
    crate::theme_file::attach_current(cx, root, &watch);
    Some(watch)
}

fn create_initial_window<S>(
    shell: &mut S,
    config: &AppConfig,
    logical_size: LogicalSize,
) -> AppResult<S::Window>
where
    S: Shell,
{
    shell
        .create_window_with(
            &config.title,
            logical_size,
            sigil_hal::WindowOpts {
                overlay: config.overlay,
                anchor: config.anchor,
            },
        )
        .map_err(Error::new)
}

struct FrameCtx<'a> {
    config: &'a AppConfig,
    shell: &'a mut WinitShell,
    window_handle: WinitWindowHandle,
    cx: &'a mut Cx,
    root: sigil_kernel::NodeId,
    layout: &'a mut LayoutEngine,
    router: &'a mut EventRouter,
    renderer: &'a mut WgpuRenderer,
    surface: &'a mut Option<WgpuSurface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
    /// G5 (DR-048 §5) — pulled from for `pump_completions_into_reactive`
    /// at frame start, before `Runtime::begin_effect`.
    async_rt: &'a mut AsyncRuntime,
    /// W5-A11Y (DR-050 §4) — inbound AT-initiated `ActionRequest`s,
    /// drained in `drain_events` and routed via `EventRouter`.
    a11y_action_rx: &'a crossbeam_channel::Receiver<accesskit::ActionRequest>,
    /// T14 (5/5 Wave 4) — dev-mode theme hot reload, drained at frame top.
    theme_watch: &'a Option<crate::theme_file::ThemeWatch>,
}

/// Run one frame under the configured [`PanicPolicy`].
///
/// `Abort` runs the frame directly, so a panic propagates and the process
/// dies (paired with `panic = "abort"` when a crash-report pipeline should
/// own the failure). `Isolate` catches a panic in this frame — a widget
/// handler, an effect, a paint closure — logs it, and resets the reactive
/// runtime to a clean resting state so the NEXT frame still runs. Without
/// this the loop simply dies, which made `PanicPolicy::Isolate` (the
/// default) a promise the app did not keep.
/// Resolve the main window's Win32 handle for muda's `init_for_hwnd`. The
/// `RawWindowHandle` enum carries every variant on all platforms, so this
/// compiles cross-platform and yields `None` off Windows — macOS uses
/// `init_for_nsapp`, which needs no handle.
fn menu_hwnd(shell: &WinitShell, handle: WinitWindowHandle) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let window = shell.window_for(handle)?;
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

fn run_frame(policy: PanicPolicy, ctx: FrameCtx<'_>) {
    match policy {
        PanicPolicy::Abort => frame_once(ctx),
        PanicPolicy::Isolate => {
            // AssertUnwindSafe: the frame borrows `&mut` UI state across the
            // boundary. After a caught panic we do not read that state as if
            // the frame had completed — we drop this frame's partial work and
            // let `recover_from_panic` reset the runtime for the next one.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| frame_once(ctx)));
            if let Err(payload) = result {
                let detail = panic_message(&*payload);
                tracing::error!("sigil-app: frame isolated after panic: {detail}");
                Runtime::recover_from_panic();
            }
        }
    }
}

/// Best-effort human-readable text from a caught panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload.downcast_ref::<&'static str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .map_or("<non-string panic payload>", String::as_str)
        },
        |s| s,
    )
}

// The DR-047 frame driver is a linear phase sequencer (Idle → Effect →
// Layout → Paint → Idle) whose ordering constraints are the contract —
// each phase reads state the previous one wrote. Splitting it would hand
// every extracted phase 8+ of `FrameCtx`'s borrows, breaching the 5-arg
// cap and hiding the ordering the comments here exist to document.
#[allow(clippy::too_many_lines)]
fn frame_once(ctx: FrameCtx<'_>) {
    let FrameCtx {
        config,
        shell,
        window_handle,
        cx,
        root,
        layout,
        router,
        renderer,
        surface,
        viewport,
        exit_requested,
        async_rt,
        a11y_action_rx,
        theme_watch,
    } = ctx;

    // T14 (5/5 Wave 4) — reload the watched theme file on save. Runs
    // before the Effect phase so the re-attached theme is visible in
    // THIS frame's layout + paint.
    if let Some(watch) = theme_watch
        && crate::theme_file::drain(cx, root, watch)
    {
        Runtime::mark_frame_dirty();
    }

    // G5 (DR-048 §5) — forward any worker-thread completion callbacks
    // into the reactive `pending_completions` queue BEFORE
    // `Runtime::begin_effect` drains it. Order matters: pump first,
    // then begin_effect — that's what makes the worker's
    // `acx.update(...)` visible to the next frame's Effect phase.
    let _moved = async_rt.pump_completions_into_reactive();

    // First live frame: install the registered native menu bar (macOS
    // `init_for_nsapp` / Windows `init_for_hwnd`). muda's item events wake
    // the loop and bridge onto `menu::events()`, drained in `menu::pump`.
    if crate::menu::has_pending() {
        let waker = shell.waker();
        crate::menu::install(menu_hwnd(shell, window_handle), move || {
            let _ = waker.wake();
        });
    }
    // Same for the system tray icon: build it on the first frame; its clicks
    // wake the loop and bridge onto `tray::events()`.
    if crate::tray::has_pending() {
        let waker = shell.waker();
        crate::tray::install(move || {
            let _ = waker.wake();
        });
    }

    // DR-047 frame driver: Idle → Effect → Layout → Paint → Idle.
    //
    // Effect phase brackets every reactive write produced during the
    // frame: drain_events runs router handlers that call signal.set /
    // entity.update / cx.dispatch_action; ensure_surface mutates cx
    // for the initial-scale path. begin_effect also drains G5 task
    // completions and any deferred actions queued in the previous
    // frame's Paint.
    Runtime::begin_effect();
    // Fire any `cx.interval` callbacks that have come due. Runs in the
    // Effect phase so the callbacks' signal writes are legal and reach
    // this frame's layout + paint via `begin_layout`'s observer fixpoint.
    // Cheap no-op when no intervals are registered. Paired with the
    // `on_idle_deadline` wake wired in `run`.
    let _ = tick_intervals();
    ensure_surface(EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        root,
        layout,
        renderer,
        surface,
        viewport,
    });
    // Drain input events during the EFFECT phase — symmetric with the
    // `pump_completions_into_reactive` hoist above. Router handlers fire
    // here (pointer_down → signal.set, key input → draft.set, Send →
    // entity.update); those writes are legal in Effect and are now
    // produced BEFORE begin_layout drains observers to fixpoint, so a
    // click/keystroke is visible in THIS frame's layout + paint instead of
    // stalling under ControlFlow::Wait until the next unrelated OS event.
    // Hit-testing uses the layout the user actually saw — the previous
    // frame's, recomputed just below.
    // Drain native menu-item clicks onto the reactive channel. Effect phase,
    // so subscriber effects (`menu::events().subscribe(...)`) run this frame.
    crate::menu::pump();
    crate::tray::pump();
    drain_events(DrainCtx {
        shell,
        cx,
        root,
        layout,
        router,
        surface,
        renderer,
        viewport,
        config,
        exit_requested,
    });
    // Layout phase: recompute Taffy boxes, now reflecting the reactive
    // writes drained above (children_reactive rebuilds, selection tints,
    // new chat bubbles). Writes during this phase still invalidate the
    // in-flight pass — discouraged by convention, not by panic.
    Runtime::begin_layout();
    let logical = logical_size_from_viewport(*viewport);
    recompute_layout(cx, root, layout, cx.scale(), logical);
    // AT action validation needs THIS frame's fresh layout/scopes, not
    // last frame's (RC-focus finding, round 2) — see
    // `event_wiring::drain_a11y_actions`'s own doc comment. Drain before
    // publish so a Focus action's effect (a plain field write) is
    // visible in this frame's tree too.
    event_wiring::drain_a11y_actions(a11y_action_rx, cx, router, layout);
    // IME sync reads focus state — after the action drain, so an
    // AT-initiated focus move updates set_ime_allowed the same frame.
    sync_ime(shell, window_handle, cx, layout, router);
    Runtime::begin_paint();
    // Change-gate + a11y publish share this dirty read (RC-focus finding 1).
    let dirty = shell.take_force_paint() || Runtime::frame_dirty();
    let _ = dirty.then(|| paint(cx, root, renderer, surface, *viewport));
    event_wiring::publish_a11y(shell, cx, router, layout, dirty);
    // end_frame transitions Paint → Effect (replay deferred writes) →
    // Idle so the next iteration enters begin_effect from a clean
    // resting state.
    Runtime::end_frame();
}

/// Drive one frame's phase transitions, calling `layout_pass` while
/// the reactive runtime is in [`DrawPhase::Layout`] and `paint_pass`
/// while it is in [`DrawPhase::Paint`].
///
/// Test-only seam that exposes the production frame driver's phase
/// shape (Idle → Effect → Layout → Paint → Idle) without requiring a
/// real winit shell, wgpu adapter, or `WgpuRenderer`. Production code
/// uses [`launch`]; the integration test in
/// `crates/sigil/tests/frame_driver.rs` drives this helper with stub
/// closures so the wiring contract (phase observable at each pass) is
/// checkable end-to-end.
///
/// Callers must invoke this from inside a [`Runtime::scope`].
///
/// [`DrawPhase::Layout`]: sigil_reactive::DrawPhase::Layout
/// [`DrawPhase::Paint`]: sigil_reactive::DrawPhase::Paint
#[doc(hidden)]
pub fn __drive_test_frame(layout_pass: impl FnOnce(), paint_pass: impl FnOnce()) {
    Runtime::begin_effect();
    Runtime::begin_layout();
    layout_pass();
    Runtime::begin_paint();
    paint_pass();
    Runtime::end_frame();
}

fn resize_surface(
    renderer: &mut WgpuRenderer,
    surface: &mut Option<WgpuSurface>,
    physical: PhysicalSize,
) {
    if let Some(surface) = surface.as_mut()
        && let Err(err) = renderer.resize(surface, physical)
    {
        tracing::warn!("sigil-app: resize failed: {err}");
    }
}

fn recompute_layout(
    cx: &mut Cx,
    root: sigil_kernel::NodeId,
    layout: &mut LayoutEngine,
    scale: Scale,
    logical: LogicalSize,
) {
    if let Err(err) = layout_for_scale(cx, root, layout, scale, logical) {
        tracing::warn!("sigil-app: layout recompute failed: {err}");
    }
}

fn layout_for_scale(
    cx: &mut Cx,
    root: sigil_kernel::NodeId,
    layout: &mut LayoutEngine,
    _scale: Scale,
    logical_viewport: LogicalSize,
) -> core::result::Result<(), sigil_layout::LayoutError> {
    layout.compute(
        cx,
        root,
        Size::fixed(logical_viewport.width, logical_viewport.height),
    )
}

const fn set_viewport(viewport: &mut (f32, f32), logical: LogicalSize) {
    viewport.0 = logical.width;
    viewport.1 = logical.height;
}

const fn logical_size_from_config(size: (u32, u32)) -> LogicalSize {
    LogicalSize::new(f32_from_u32(size.0), f32_from_u32(size.1))
}

const fn logical_size_from_viewport(viewport: (f32, f32)) -> LogicalSize {
    LogicalSize::new(viewport.0, viewport.1)
}

fn clamp_physical_size(size: PhysicalSize) -> PhysicalSize {
    PhysicalSize::new(size.width.max(1), size.height.max(1))
}

#[allow(clippy::cast_precision_loss)]
const fn f32_from_u32(v: u32) -> f32 {
    v as f32
}

#[allow(clippy::cast_possible_truncation)]
fn scale_from_f64(scale: f64) -> Scale {
    Scale::new(scale as f32)
}

fn paint(
    cx: &Cx,
    root: sigil_kernel::NodeId,
    renderer: &mut WgpuRenderer,
    surface: &mut Option<WgpuSurface>,
    viewport: (f32, f32),
) {
    let Some(surface) = surface.as_mut() else {
        return;
    };
    let scene = emit_scene(cx, root);
    // Transparent (overlay) surfaces clear fully transparent so unpainted
    // regions show the desktop behind them; normal windows keep the
    // opaque base.
    let clear = if renderer.is_transparent() {
        Color::rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        Color::rgba(0.03, 0.04, 0.05, 1.0)
    };
    if let Err(err) = renderer.render_command_scene(surface, &scene, clear, viewport) {
        tracing::warn!("sigil-app: render_command_scene failed: {err}");
        // The drawable still has no fresh pixels and this frame consumed
        // the dirty flag — re-arm it so the next frame actually retries
        // instead of change-gating away the retry forever.
        Runtime::mark_frame_dirty();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::fmt;

    use sigil_hal::{LogicalSize, PhysicalSize, Scale, Shell, ShellEvent};
    use sigil_kernel::{Cx, container};
    use sigil_layout::prelude::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
    use sigil_reactive::Runtime;

    use crate::config::AppConfig;
    use crate::launch::{create_initial_window, layout_for_scale};
    use crate::text_measure::DefaultTextMeasure;

    #[derive(Debug)]
    struct MockError;

    impl fmt::Display for MockError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("mock shell error")
        }
    }

    impl std::error::Error for MockError {}

    #[derive(Clone)]
    struct MockWaker;

    #[derive(Debug, PartialEq)]
    enum CreateCall {
        Physical(String, PhysicalSize),
        Logical(String, LogicalSize),
        With(String, LogicalSize, sigil_hal::WindowOpts),
    }

    #[derive(Default)]
    struct MockShell {
        calls: Vec<CreateCall>,
    }

    impl Shell for MockShell {
        type Error = MockError;
        type Waker = MockWaker;
        type Window = usize;

        fn waker(&self) -> Self::Waker {
            MockWaker
        }

        fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
            Ok(())
        }

        fn create_window(
            &mut self,
            title: &str,
            initial_size: PhysicalSize,
        ) -> Result<Self::Window, Self::Error> {
            self.calls
                .push(CreateCall::Physical(title.to_owned(), initial_size));
            Ok(self.calls.len() - 1)
        }

        fn create_window_logical(
            &mut self,
            title: &str,
            initial: LogicalSize,
        ) -> Result<Self::Window, Self::Error> {
            self.calls
                .push(CreateCall::Logical(title.to_owned(), initial));
            Ok(self.calls.len() - 1)
        }

        fn create_window_with(
            &mut self,
            title: &str,
            initial: LogicalSize,
            opts: sigil_hal::WindowOpts,
        ) -> Result<Self::Window, Self::Error> {
            self.calls
                .push(CreateCall::With(title.to_owned(), initial, opts));
            Ok(self.calls.len() - 1)
        }

        fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
            Vec::new().into_iter()
        }

        fn run<F>(self, _frame: F) -> Result<(), Self::Error>
        where
            F: FnMut(&mut Self) + 'static,
        {
            Ok(())
        }
    }

    #[test]
    fn initial_window_passes_logical_size_and_default_opts() {
        let mut shell = MockShell::default();
        let config = AppConfig::new("test").size(720, 480);
        let logical = LogicalSize::new(720.0, 480.0);

        let handle = create_initial_window(&mut shell, &config, logical).unwrap();

        assert_eq!(handle, 0);
        assert_eq!(
            shell.calls,
            vec![CreateCall::With(
                "test".to_owned(),
                logical,
                sigil_hal::WindowOpts::default()
            )]
        );
    }

    #[test]
    fn initial_window_threads_overlay_opts() {
        let mut shell = MockShell::default();
        let anchor = sigil_hal::Anchor::BottomCenter { margin: 64.0 };
        let config = AppConfig::new("pill")
            .size(200, 48)
            .overlay(true)
            .anchor(anchor);
        let logical = LogicalSize::new(200.0, 48.0);

        create_initial_window(&mut shell, &config, logical).unwrap();

        assert_eq!(
            shell.calls,
            vec![CreateCall::With(
                "pill".to_owned(),
                logical,
                sigil_hal::WindowOpts {
                    overlay: true,
                    anchor: Some(anchor),
                }
            )]
        );
    }

    #[test]
    fn layout_for_scale_keeps_logical_rects_stable() {
        Runtime::scope("test", || {
            let mut cx = Cx::new_root();
            let root = cx.add(container().layout(LayoutStyle::block().fixed_size(720.0, 480.0)));
            let logical = LogicalSize::new(720.0, 480.0);
            let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

            layout_for_scale(&mut cx, root, &mut layout, Scale::ONE, logical).unwrap();
            let one_x = layout.result(root).unwrap().rect();

            layout_for_scale(&mut cx, root, &mut layout, Scale::new(2.0), logical).unwrap();
            let two_x = layout.result(root).unwrap().rect();

            assert_eq!(one_x, two_x);
            assert_eq!(two_x.width.to_bits(), 720.0_f32.to_bits());
            assert_eq!(two_x.height.to_bits(), 480.0_f32.to_bits());
        });
    }
}
