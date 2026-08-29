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
    let config = AppConfig {
        user_zoom: if config.user_zoom > 0.0 {
            config.user_zoom
        } else {
            1.0
        },
        ..config
    };
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

    let mut viewport = (
        logical_size.width / config.user_zoom,
        logical_size.height / config.user_zoom,
    );
    host.layout(viewport).map_err(Error::new)?;

    let mut router = EventRouter::new();
    let a11y_action_rx = shell.a11y_action_receiver();
    let mut renderer = WgpuRenderer::new()?;
    renderer.set_transparent(config.overlay || config.frameless);
    let mut surface: Option<WgpuSurface> = None;
    let mut exit_requested = false;
    let mut frame_state = FrameState::default();

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
        window.request_redraw();
    }
}

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

pub fn install_g5_executor(async_rt: &AsyncRuntime) {
    let executor: Arc<dyn TaskExecutor> = Arc::new(async_rt.tokio_executor());
    let dispatcher: Arc<dyn MainThreadDispatcher> = Arc::new(async_rt.reactive_bridge());
    Runtime::install_executor(executor, dispatcher);
}

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
                user_zoom: ((config.user_zoom - 1.0).abs() > f32::EPSILON)
                    .then_some(config.user_zoom),
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
    async_rt: &'a mut AsyncRuntime,
    a11y_action_rx: &'a crossbeam_channel::Receiver<accesskit::ActionRequest>,
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
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| frame_once(ctx)));
            if let Err(payload) = result {
                let detail = panic_message(&*payload);
                tracing::error!("sigil-app: frame isolated after panic: {detail}");
                Runtime::recover_from_panic();
            }
        }
    }
}

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

#[allow(
    clippy::too_many_lines,
    reason = "one sequential pass; splitting would scatter the ordering it documents"
)]
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

    if let Some(watch) = theme_watch
        && let Some(root) = host.root()
        && crate::theme_file::drain(host.cx(), root, watch)
    {
        Runtime::mark_frame_dirty();
    }

    let _moved = async_rt.pump_completions_into_reactive();

    if crate::menu::has_pending() {
        let waker = shell.waker();
        crate::menu::install(menu_hwnd(shell, window_handle), move || {
            let _ = waker.wake();
        });
    }
    if crate::tray::has_pending() {
        let waker = shell.waker();
        crate::tray::install(move || {
            let _ = waker.wake();
        });
    }

    sigil_profile::begin_frame();
    let effect = sigil_profile::span("effect");
    Runtime::begin_effect();
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
    let animation_advanced = frame_state.animation_clock.tick_active(Instant::now());
    if animation_advanced {
        sigil_profile::interact("animation");
    }
    drop(effect);
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
    if let Some((cx, layout, _)) = host.tree() {
        event_wiring::drain_a11y_actions(a11y_action_rx, cx, router, layout);
        sync_ime(shell, window_handle, cx, layout, router);
    }
    Runtime::begin_paint();
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
    Runtime::end_frame();
    sigil_profile::tick_frame();
}

fn run_layout_if_dirty(dirty: bool, layout_pass: impl FnOnce()) -> bool {
    if dirty {
        layout_pass();
    }
    dirty
}

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

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
const fn f32_from_u32(v: u32) -> f32 {
    v as f32
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
fn scale_from_f64(scale: f64) -> Scale {
    Scale::new(scale as f32)
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
