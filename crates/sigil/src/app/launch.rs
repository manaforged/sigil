use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::animation_runtime::active_count as active_animation_count;
#[cfg(test)]
use crate::animation_runtime::tick as tick_animations;
use crate::events::EventRouter;
use crate::hal::{LogicalSize, PhysicalSize, Scale, Shell};
use crate::kernel::Cx;
#[cfg(test)]
use crate::layout::LayoutEngine;
#[cfg(test)]
use crate::layout::prelude::Size;
use crate::reactive::{Runtime, next_interval_deadline};
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use crate::app::config::{AppConfig, PanicPolicy};
use crate::app::embed::Host;
use crate::app::error::{AppResult, Error};
use crate::app::exit::clear_exit;
use crate::app::tree_root::TreeRoot;

mod backend;
mod event_wiring;
mod frame;
#[path = "scene_cache.rs"]
mod scene_cache;

use backend::{Backend, Surface};
use frame::frame_once;
#[cfg(test)]
use frame::{begin_effect, run_layout_if_dirty};
use scene_cache::SceneCache;

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
    let failure = Rc::new(Cell::new(None));
    let launch_failure = Rc::clone(&failure);
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
                if let Err(error) = run_frame(panic_policy, frame) {
                    Runtime::recover_from_panic();
                    launch_failure.set(Some(error));
                    shell.request_exit();
                    return;
                }
                request_animation_frame(shell, window_handle);
            })
            .map_err(Error::new)
            .and_then(|()| failure.take().map_or(Ok(()), Err)),
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
    Ok(async_rt)
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
    fn elapsed(&mut self, now: Instant) -> f32 {
        if active_animation_count() == 0 {
            self.previous = None;
            return NOMINAL_FRAME_SECS;
        }
        self.previous
            .replace(now)
            .map_or(NOMINAL_FRAME_SECS, |previous| {
                now.saturating_duration_since(previous)
                    .as_secs_f32()
                    .min(MAX_ANIMATION_FRAME_SECS)
            })
    }

    #[cfg(test)]
    fn tick_active(&mut self, now: Instant) -> bool {
        if active_animation_count() == 0 {
            self.previous = None;
            return false;
        }
        let dt = self.elapsed(now);
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

fn run_frame(policy: PanicPolicy, ctx: FrameCtx<'_>) -> AppResult<()> {
    match policy {
        PanicPolicy::Abort => frame_once(ctx),
        PanicPolicy::Isolate => {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| frame_once(ctx)));
            result.unwrap_or_else(|payload| {
                let detail = crate::tasks::panic_message(&*payload);
                let origin = Runtime::panic_origin_info()
                    .map_or_else(String::new, |(name, location)| {
                        format!(" [effect {name} {location}]")
                    });
                tracing::error!("sigil-app: frame isolated after panic: {detail}{origin}");
                Runtime::recover_from_panic();
                Ok(())
            })
        }
    }
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
