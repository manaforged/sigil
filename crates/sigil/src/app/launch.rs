use std::time::Instant;

use crate::animation_runtime::{active_count as active_animation_count, tick as tick_animations};
use crate::events::EventRouter;
use crate::hal::{LogicalSize, PhysicalSize, Scale, Shell};
use crate::kernel::Cx;
#[cfg(test)]
use crate::layout::LayoutEngine;
#[cfg(test)]
use crate::layout::prelude::Size;
use crate::reactive::{Runtime, next_interval_deadline, tick_intervals};
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use crate::app::config::{AppConfig, PanicPolicy};
use crate::app::embed::Host;
use crate::app::error::{AppResult, Error};
use crate::app::exit::{clear_exit, take_exit};
use crate::app::tree_root::TreeRoot;

mod backend;
mod event_wiring;
#[path = "scene_cache.rs"]
mod scene_cache;

use backend::{Backend, Surface};
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
    let mut config = AppConfig {
        user_zoom: if config.user_zoom > 0.0 {
            config.user_zoom
        } else {
            1.0
        },
        ..config
    };
    crate::text::prefer_system_fonts();
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
    let mut renderer = Backend::from_env()?;
    renderer.set_transparent(config.overlay || config.frameless);
    let mut surface: Option<Surface> = None;
    let mut exit_requested = false;
    clear_exit();
    let mut frame_state = FrameState::default();

    shell.on_idle_deadline(next_interval_deadline);

    let panic_policy = config.panic_policy;
    finish(
        shell
            .run(move |shell| {
                let frame = FrameCtx {
                    config: &mut config,
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
    crate::profile::dump_if_enabled();
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
    let waker = shell.waker();
    async_rt.set_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    async_rt.install_current();
    install_g5_executor(&async_rt);
    Ok(async_rt)
}

pub fn install_g5_executor(async_rt: &AsyncRuntime) {
    async_rt.install_executor();
}

fn start_theme_watch(
    config: &AppConfig,
    cx: &mut Cx,
    root: crate::kernel::NodeId,
) -> Option<crate::app::theme_file::ThemeWatch> {
    let watch = crate::app::theme_file::start(config.theme_file.as_deref())?;
    crate::app::theme_file::attach_current(cx, root, &watch);
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
            crate::hal::WindowOpts {
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
    config: &'a mut AppConfig,
    shell: &'a mut WinitShell,
    window_handle: WinitWindowHandle,
    host: &'a mut Host,
    router: &'a mut EventRouter,
    renderer: &'a mut Backend,
    surface: &'a mut Option<Surface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
    async_rt: &'a mut AsyncRuntime,
    a11y_action_rx: &'a crossbeam_channel::Receiver<accesskit::ActionRequest>,
    theme_watch: &'a Option<crate::app::theme_file::ThemeWatch>,
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

#[cfg(all(feature = "menu", target_os = "windows"))]
fn menu_hwnd(shell: &WinitShell, handle: WinitWindowHandle) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let window = shell.window_for(handle)?;
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

#[cfg(all(feature = "menu", not(target_os = "windows")))]
const fn menu_hwnd(_shell: &WinitShell, _handle: WinitWindowHandle) -> Option<isize> {
    None
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

    crate::profile::begin_frame();
    let effect = crate::profile::span("effect");
    begin_effect(async_rt);

    if let Some(watch) = theme_watch
        && let Some(root) = host.root()
        && crate::app::theme_file::drain(host.cx(), root, watch)
    {
        Runtime::mark_frame_dirty();
    }

    if let Some(theme) = crate::app::appearance::take()
        && let Some(root) = host.root()
        && let Some(node) = host.cx().get_mut(root)
    {
        node.attach(theme);
        Runtime::mark_frame_dirty();
    }

    #[cfg(feature = "menu")]
    if crate::app::menu::has_pending() {
        let waker = shell.waker();
        if let Err(error) = crate::app::menu::install(menu_hwnd(shell, window_handle), move || {
            let _ = waker.wake();
        }) {
            tracing::error!("sigil-app: cannot install native menu: {error}");
        }
    }
    #[cfg(feature = "tray")]
    if crate::app::tray::has_pending() {
        let waker = shell.waker();
        crate::app::tray::install(move || {
            let _ = waker.wake();
        });
    }

    if tick_intervals() > 0 {
        crate::profile::interact("interval");
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
    #[cfg(feature = "menu")]
    crate::app::menu::pump();
    #[cfg(feature = "tray")]
    crate::app::tray::pump();
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
        event_wiring::drain_a11y_actions(a11y_action_rx, cx, router, layout);
    }
    if take_exit() {
        *exit_requested = true;
    }
    let animation_advanced = frame_state.animation_clock.tick_active(Instant::now());
    if animation_advanced {
        crate::profile::interact("animation");
    }
    drop(effect);
    Runtime::begin_layout();
    let force_paint = shell.take_force_paint();
    let layout_dirty = Runtime::frame_dirty();
    {
        let _layout = crate::profile::span("layout");
        run_layout_if_dirty(layout_dirty, || {
            if let Err(err) = host.layout(*viewport) {
                tracing::warn!("sigil-app: layout recompute failed: {err}");
            }
        });
    }
    if let Some((cx, layout, _)) = host.tree() {
        sync_ime(shell, window_handle, cx, layout, router);
    }
    Runtime::begin_paint();
    let paint_dirty = Runtime::frame_dirty();
    let paint_needed = force_paint || paint_dirty;
    let allow_scene_replay = force_paint && !paint_dirty && !animation_advanced;
    if allow_scene_replay {
        crate::profile::interact("expose");
    }
    {
        let _paint = crate::profile::span("paint");
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
        let _a11y = crate::profile::span("a11y");
        if let Some((cx, layout, _)) = host.tree() {
            event_wiring::publish_a11y(shell, cx, router, layout, paint_needed);
        }
    }
    Runtime::end_frame();
    crate::profile::tick_frame();
}

fn begin_effect(async_rt: &mut AsyncRuntime) {
    Runtime::begin_effect();
    let _drained = async_rt.pump();
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

fn resize_surface(renderer: &mut Backend, surface: &mut Option<Surface>, physical: PhysicalSize) {
    if let Some(surface) = surface.as_mut()
        && let Err(err) = renderer.resize(surface, physical)
    {
        tracing::warn!("sigil-app: resize failed: {err}");
    }
}

#[cfg(test)]
fn layout_for_scale(
    cx: &mut Cx,
    root: crate::kernel::NodeId,
    layout: &mut LayoutEngine,
    _scale: Scale,
    logical_viewport: LogicalSize,
) -> core::result::Result<(), crate::layout::LayoutError> {
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
