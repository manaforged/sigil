//! `sigil::launch` — the one-call entry point.
//!
//! Owns the full frame pipeline: reactive scope → shell/window → renderer
//! → lazy surface → [`Host`](crate::Host) → event drain → scene emit → paint.
//! Every failure mode listed in `phase-08a-ergonomics-pass.md`'s
//! bulletproof table becomes unreachable at this layer.

use std::sync::Arc;
use std::time::Instant;

use sigil_animation_runtime::{active_count as active_animation_count, tick as tick_animations};
use sigil_events::EventRouter;
use sigil_hal::{LogicalSize, PhysicalSize, Renderer as _, Scale, Shell};
use sigil_kernel::Cx;
#[cfg(test)]
use sigil_layout::LayoutEngine;
#[cfg(test)]
use sigil_layout::prelude::Size;
use sigil_reactive::{
    MainThreadDispatcher, Runtime, TaskExecutor, next_interval_deadline, tick_intervals,
};
use sigil_renderer::{WgpuRenderer, WgpuSurface};
use sigil_shell::{WinitShell, WinitWindowHandle};
use sigil_tasks::AsyncRuntime;

use crate::config::{AppConfig, PanicPolicy};
use crate::embed::Host;
use crate::error::{AppResult, Error};
use crate::tree_root::TreeRoot;

mod event_wiring;
#[path = "scene_cache.rs"]
mod scene_cache;

use event_wiring::{DrainCtx, EnsureSurfaceCtx, drain_events, ensure_surface, sync_ime};
use scene_cache::{PaintRequest, SceneCache};

/// Launch a sigil app.
///
/// `config` accepts `()`, `&str`, `(title, w, h)`, or `AppConfig`.
/// `tree` is a closure, `text(...)`, `image(...)`, a `container()`,
/// or [`crate::root`] wrapping a widget.
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

#[allow(clippy::too_many_lines, reason = "launch is one linear setup")]
fn run<T: TreeRoot>(config: AppConfig, tree: T) -> AppResult<()> {
    // A non-positive zoom is a caller bug, not a layout death sentence.
    let config = AppConfig {
        user_zoom: if config.user_zoom > 0.0 {
            config.user_zoom
        } else {
            1.0
        },
        ..config
    };
    // Live desktop apps use the platform UI families by default. A consumer
    // that installed bundled/custom faces before launch keeps that choice.
    sigil_text::prefer_system_fonts();
    let logical_size = logical_size_from_config(config.size);

    let mut shell = if config.overlay {
        WinitShell::new_accessory()
    } else {
        WinitShell::new()
    }
    .map_err(Error::new)?;
    let window_handle = create_initial_window(&mut shell, &config, logical_size)?;

    let mut async_rt = install_async_runtime(&shell)?;

    let mut host = Host::new();
    host.mount(tree);
    let root = host.root().expect("TreeRoot::install returns a node");
    let theme_watch = start_theme_watch(&config, host.cx(), root);

    // The viewport is sigil-logical: the window keeps `logical_size` in
    // winit terms, but content lays out zoom-times smaller so the composed
    // scale paints it back to the same physical size (webview-zoom rules).
    let mut viewport = (
        logical_size.width / config.user_zoom,
        logical_size.height / config.user_zoom,
    );
    host.layout(viewport).map_err(Error::new)?;

    let mut router = EventRouter::new();
    // W5-A11Y (DR-050 §1) — clone the inbound `ActionRequest` receiver
    // before `shell` is moved into `Shell::run` below; the frame loop
    // drains it every frame via `drain_a11y_actions`.
    let a11y_action_rx = shell.a11y_action_receiver();
    let mut renderer = WgpuRenderer::new()?;
    renderer.set_transparent(config.overlay || config.frameless);
    let mut surface: Option<WgpuSurface> = None;
    let mut exit_requested = false;
    let mut frame_state = FrameState::default();

    // Wake the loop when the soonest `cx.interval` is due, so periodic
    // callbacks (clocks, live metrics) fire without waiting on an OS
    // input event. The shell turns this into `ControlFlow::WaitUntil`;
    // `None` (no intervals registered) falls back to `Wait` at zero cost.
    // Paired with the `tick_intervals` drain in `frame_once`.
    shell.on_idle_deadline(next_interval_deadline);

    let panic_policy = config.panic_policy;
    finish(
        shell
            .run(move |shell| {
                let frame = FrameCtx {
                    config: &config,
                    shell,
                    window_handle,
                    host: &mut host,
                    router: &mut router,
                    renderer: &mut renderer,
                    surface: &mut surface,
                    viewport: &mut viewport,
                    exit_requested: &mut exit_requested,
                    async_rt: &mut async_rt,
                    a11y_action_rx: &a11y_action_rx,
                    theme_watch: &theme_watch,
                    frame_state: &mut frame_state,
                };
                run_frame(panic_policy, frame);
                request_animation_frame(shell, window_handle);
            })
            .map_err(Error::new),
    )
}

fn finish(result: AppResult<()>) -> AppResult<()> {
    sigil_profile::dump_if_enabled();
    result
}

fn request_animation_frame(shell: &WinitShell, handle: WinitWindowHandle) {
    if active_animation_count() > 0
        && let Some(window) = shell.window_for(handle)
    {
        // `AutoVsync` paces presentation; requesting the successor here keeps
        // the event loop asleep whenever no motion is active.
        window.request_redraw();
    }
}

/// Build the Tokio runtime, install [`sigil_tasks::spawn`] on this
/// thread, and wire the worker→main bridge so a completion wakes
/// the event loop and paints on the next frame.
fn install_async_runtime(shell: &WinitShell) -> AppResult<AsyncRuntime> {
    let mut async_rt = AsyncRuntime::new().map_err(Error::new)?;
    async_rt.install_current();
    let waker = shell.waker();
    async_rt.set_bridge_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    install_g5_executor(&async_rt);
    Ok(async_rt)
}

/// Wire hidden `EntityCx::spawn` to the supplied async runtime.
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
                frameless: config.frameless,
                anchor: config.anchor,
                user_zoom: (config.user_zoom != 1.0).then_some(config.user_zoom),
            },
        )
        .map_err(Error::new)
}

struct FrameCtx<'a> {
    config: &'a AppConfig,
    shell: &'a mut WinitShell,
    window_handle: WinitWindowHandle,
    host: &'a mut Host,
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
    frame_state: &'a mut FrameState,
}

#[derive(Default)]
struct FrameState {
    animation_clock: AnimationClock,
    scene_cache: SceneCache,
}

const NOMINAL_FRAME_SECS: f32 = 1.0 / 60.0;
const MAX_ANIMATION_FRAME_SECS: f32 = 0.1;

#[derive(Default)]
struct AnimationClock {
    previous: Option<Instant>,
}

impl AnimationClock {
    fn tick_active(&mut self, now: Instant) -> bool {
        if active_animation_count() == 0 {
            self.previous = None;
            return false;
        }
        let dt = self
            .previous
            .replace(now)
            .map_or(NOMINAL_FRAME_SECS, |previous| {
                now.saturating_duration_since(previous)
                    .as_secs_f32()
                    .min(MAX_ANIMATION_FRAME_SECS)
            });
        tick_animations(dt);
        if active_animation_count() == 0 {
            self.previous = None;
        }
        true
    }
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
        host,
        router,
        renderer,
        surface,
        viewport,
        exit_requested,
        async_rt,
        a11y_action_rx,
        theme_watch,
        frame_state,
    } = ctx;

    // T14 (5/5 Wave 4) — reload the watched theme file on save. Runs
    // before the Effect phase so the re-attached theme is visible in
    // THIS frame's layout + paint.
    if let Some(watch) = theme_watch
        && let Some(root) = host.root()
        && crate::theme_file::drain(host.cx(), root, watch)
    {
        Runtime::mark_frame_dirty();
    }

    // G5 (DR-048 §5) — forward any worker-thread completion callbacks
    // into the reactive `pending_completions` queue BEFORE
    // `Runtime::begin_effect` drains it. Order matters: pump first,
    // then begin_effect — that's what makes the worker's
    // `acx.update(...)` visible to the next frame's Effect phase.
    let _moved = async_rt.pump_completions_into_reactive();

    // Install or replace a pending native menu bar (macOS
    // `init_for_nsapp` / Windows `init_for_hwnd`). muda's item events wake
    // the loop and bridge onto `menu::events()`, drained in `menu::pump`.
    if crate::menu::has_pending() {
        let waker = shell.waker();
        crate::menu::install(menu_hwnd(shell, window_handle), move || {
            let _ = waker.wake();
        });
    }
    // Same for the system tray icon: build or replace when pending; clicks
    // wake the loop and bridge onto `tray::events()`.
    if crate::tray::has_pending() {
        let waker = shell.waker();
        crate::tray::install(move || {
            let _ = waker.wake();
        });
    }

    sigil_profile::begin_frame();
    // DR-047 frame driver: Idle → Effect → Layout → Paint → Idle.
    //
    // Effect phase brackets every reactive write produced during the
    // frame: drain_events runs router handlers that call signal.set /
    // entity.update / cx.dispatch_action; ensure_surface mutates cx
    // for the initial-scale path. begin_effect also drains G5 task
    // completions and any deferred actions queued in the previous
    // frame's Paint.
    let effect = sigil_profile::span("effect");
    Runtime::begin_effect();
    // Fire any `cx.interval` callbacks that have come due. Runs in the
    // Effect phase so the callbacks' signal writes are legal and reach
    // this frame's layout + paint via `begin_layout`'s observer fixpoint.
    // Cheap no-op when no intervals are registered. Paired with the
    // `on_idle_deadline` wake wired in `run`.
    if tick_intervals() > 0 {
        sigil_profile::interact("interval");
    }
    ensure_surface(EnsureSurfaceCtx {
        shell,
        window_handle,
        cx: host.cx(),
        renderer,
        surface,
        viewport,
        user_zoom: config.user_zoom,
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
    if let Some((cx, layout, _)) = host.tree() {
        drain_events(DrainCtx {
            shell,
            window_handle,
            cx,
            layout,
            router,
            surface,
            renderer,
            viewport,
            config,
            exit_requested,
        });
    }
    // Advance motion while writes are legal, before layout reads animated
    // geometry. The frame callback requests a successor redraw only while the
    // runtime still has moving animators, so idle windows consume no frame loop.
    let animation_advanced = frame_state.animation_clock.tick_active(Instant::now());
    if animation_advanced {
        sigil_profile::interact("animation");
    }
    drop(effect);
    // Layout phase: recompute layout boxes, now reflecting the reactive
    // writes drained above (children_reactive rebuilds, selection tints,
    // new chat bubbles). Writes during this phase still invalidate the
    // in-flight pass — discouraged by convention, not by panic.
    Runtime::begin_layout();
    let force_paint = shell.take_force_paint();
    let layout_dirty = Runtime::frame_dirty();
    {
        let _layout = sigil_profile::span("layout");
        run_layout_if_dirty(layout_dirty, || {
            if let Err(err) = host.layout(*viewport) {
                tracing::warn!("sigil-app: layout recompute failed: {err}");
            }
        });
    }
    // AT action validation needs THIS frame's fresh layout/scopes, not
    // last frame's (RC-focus finding, round 2) — see
    // `event_wiring::drain_a11y_actions`'s own doc comment. Drain before
    // publish so a Focus action's effect (a plain field write) is
    // visible in this frame's tree too.
    if let Some((cx, layout, _)) = host.tree() {
        event_wiring::drain_a11y_actions(a11y_action_rx, cx, router, layout);
        // IME sync reads focus state — after the action drain, so an
        // AT-initiated focus move updates set_ime_allowed the same frame.
        sync_ime(shell, window_handle, cx, layout, router);
    }
    Runtime::begin_paint();
    // Change-gate + a11y publish share this dirty read (RC-focus finding 1).
    let paint_dirty = Runtime::frame_dirty();
    let paint_needed = force_paint || paint_dirty;
    let allow_scene_replay = force_paint && !paint_dirty && !animation_advanced;
    if allow_scene_replay {
        sigil_profile::interact("expose");
    }
    {
        let _paint = sigil_profile::span("paint");
        let _ = paint_needed.then(|| {
            if let Some((cx, _, root)) = host.tree() {
                frame_state.scene_cache.paint(PaintRequest {
                    cx,
                    root,
                    renderer,
                    surface,
                    viewport: *viewport,
                    allow_replay: allow_scene_replay,
                });
            }
        });
    }
    {
        let _a11y = sigil_profile::span("a11y");
        if let Some((cx, layout, _)) = host.tree() {
            event_wiring::publish_a11y(shell, cx, router, layout, paint_needed);
        }
    }
    // end_frame transitions Paint → Effect (replay deferred writes) →
    // Idle so the next iteration enters begin_effect from a clean
    // resting state.
    Runtime::end_frame();
    sigil_profile::tick_frame();
}

fn run_layout_if_dirty(dirty: bool, layout_pass: impl FnOnce()) -> bool {
    if dirty {
        layout_pass();
    }
    dirty
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

#[cfg(test)]
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

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
