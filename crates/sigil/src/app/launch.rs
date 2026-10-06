use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::active_count as active_animation_count;
#[cfg(test)]
use crate::geometry::Size;
use crate::geometry::{LogicalSize, PhysicalSize, Scale};
use crate::hal::Shell;
#[cfg(test)]
use crate::layout::LayoutEngine;
use crate::reactive::{Owner, Runtime, next_interval_deadline};
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use crate::app::config::{AppConfig, PanicPolicy};
use crate::app::error::{AppResult, Error};
use crate::app::exit::clear_exit;
use crate::app::window::allocate_window_id;
use crate::kernel::IntoNode;

pub(super) mod backend;
mod clock;
mod event_wiring;
mod frame;
mod lifecycle;
mod record;
#[path = "scene_cache.rs"]
mod scene_cache;
mod tick;
#[cfg(target_arch = "wasm32")]
mod web;

use backend::{Backend, Surface};
#[cfg(test)]
use clock::AnimationClock;
use frame::frame_once;
#[cfg(test)]
use frame::{begin_effect, run_layout_if_dirty};
use record::{WindowRecord, Windows};
use tick::{FrameCtx, LoopState};

pub(crate) fn launch<C, F, N>(config: C, tree: F) -> AppResult<()>
where
    C: Into<AppConfig>,
    F: Fn() -> N + 'static,
    N: IntoNode,
{
    let config = config.into();
    let owner = Owner::new("sigil-app", None);
    let result = owner.with(|| run(&config, tree, owner));
    #[cfg(not(target_arch = "wasm32"))]
    owner.dispose();
    #[cfg(target_arch = "wasm32")]
    if let Err(error) = &result {
        web::report(error);
    }
    result
}

fn run<F, N>(config: &AppConfig, tree: F, owner: Owner) -> AppResult<()>
where
    F: Fn() -> N + 'static,
    N: IntoNode,
{
    crate::app::window::reset();
    crate::text::prefer_system_fonts();
    let mut shell = if config.overlay {
        WinitShell::new_accessory()
    } else {
        WinitShell::new()
    }
    .map_err(Error::window)?;
    #[cfg(target_arch = "wasm32")]
    shell.set_canvas(config.canvas.clone());
    let async_rt = install_async_runtime(&shell, config.task_workers)?;
    install_font_wake(&shell);
    let windows = initial_windows(&mut shell, config, tree)?;
    let _theme_watch = crate::app::launch_theme(config.theme_file.as_deref());
    let mut renderer = Backend::from_env()?;
    #[cfg(target_arch = "wasm32")]
    renderer.request_device({
        let waker = shell.waker();
        move || drop(WinitShell::wake(&waker))
    });
    renderer.set_transparent(config.overlay || config.frameless);
    clear_exit();
    shell.on_idle_deadline(next_interval_deadline);
    let state = LoopState::new(windows, renderer, async_rt, config.panic_policy);
    drive(shell, state, owner)
}

fn initial_windows<F, N>(shell: &mut WinitShell, config: &AppConfig, tree: F) -> AppResult<Windows>
where
    F: Fn() -> N + 'static,
    N: IntoNode,
{
    let mut config = config.clone();
    if config.user_zoom <= 0.0 {
        config.user_zoom = 1.0;
    }
    let handle = create_initial_window(shell, &config, logical_size_from_config(config.size))?;
    let record = WindowRecord::new(
        allocate_window_id(),
        handle,
        config,
        Box::new(move |host: &mut crate::app::embed::Host| host.mount_app(tree)),
    )?;
    let mut windows = Windows::default();
    windows.insert(record);
    Ok(windows)
}

fn drive(shell: WinitShell, mut state: LoopState, owner: Owner) -> AppResult<()> {
    #[cfg(feature = "inspect")]
    let inspect = crate::app::inspect::install_with_wake({
        let waker = shell.waker();
        move || drop(WinitShell::wake(&waker))
    });
    let failure = Rc::new(Cell::new(None));
    let launch_failure = Rc::clone(&failure);
    finish(
        shell
            .run(move |shell| {
                owner.with(|| {
                    #[cfg(feature = "inspect")]
                    if let Some(inspect) = &inspect {
                        state.service_inspect(inspect);
                    }
                    if let Err(error) = state.tick(shell) {
                        Runtime::recover_from_panic();
                        #[cfg(target_arch = "wasm32")]
                        web::report(&error);
                        launch_failure.set(Some(error));
                        shell.request_exit();
                    }
                });
            })
            .map_err(Error::window)
            .and_then(|()| failure.take().map_or(Ok(()), Err)),
    )
}

fn finish(result: AppResult<()>) -> AppResult<()> {
    if cfg!(not(target_arch = "wasm32")) {
        crate::profile::dump_if_enabled();
    }
    result
}

fn request_animation_frame(shell: &WinitShell, handle: Option<WinitWindowHandle>) {
    if active_animation_count() > 0
        && let Some(window) = handle.and_then(|handle| shell.window_for(handle))
    {
        window.request_redraw();
    }
}

fn install_font_wake(shell: &WinitShell) {
    let waker = shell.waker();
    crate::text::on_fonts_changed(move || drop(WinitShell::wake(&waker)));
}

fn install_async_runtime(
    shell: &WinitShell,
    workers: crate::tasks::TaskWorkers,
) -> AppResult<AsyncRuntime> {
    let mut async_rt = AsyncRuntime::with_workers(workers).map_err(Error::task)?;
    let waker = shell.waker();
    async_rt.set_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    async_rt.install_current();
    Ok(async_rt)
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
            crate::app::WindowOpts {
                overlay: config.overlay,
                frameless: config.frameless,
                anchor: config.anchor,
                user_zoom: ((config.user_zoom - 1.0).abs() > f32::EPSILON)
                    .then_some(config.user_zoom),
                min_size: config.min_size.map(logical_size_from_config),
                fixed_size: config.resize == crate::app::config::Resize::Fixed,
            },
        )
        .map_err(Error::window)
}

#[cfg(all(any(feature = "menu", feature = "media"), target_os = "windows"))]
fn menu_hwnd(shell: &WinitShell, handle: WinitWindowHandle) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let window = shell.window_for(handle)?;
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

#[cfg(all(any(feature = "menu", feature = "media"), not(target_os = "windows")))]
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

#[cfg(any(test, feature = "kittest"))]
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
    cx: &mut crate::kernel::Cx,
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
