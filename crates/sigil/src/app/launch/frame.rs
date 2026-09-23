use std::cell::RefCell;
use std::time::Instant;

use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::app::error::{AppResult, Error};
use crate::app::{FrameHooks, FrameInfo};
use crate::events::EventRouter;
#[cfg(feature = "menu")]
use crate::hal::Shell;
use crate::reactive::Runtime;
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use super::FrameCtx;
use super::backend::{Backend, Surface};
use super::event_wiring::{
    self, DrainCtx, EnsureSurfaceCtx, drain_events, ensure_surface, sync_ime,
};
#[cfg(feature = "menu")]
use super::menu_hwnd;
use super::scene_cache::PaintRequest;

struct FrameEnv<'a> {
    config: &'a mut AppConfig,
    shell: &'a mut WinitShell,
    window_handle: WinitWindowHandle,
    renderer: &'a mut Backend,
    surface: &'a mut Option<Surface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
    async_rt: &'a mut AsyncRuntime,
    a11y_action_rx: &'a crossbeam_channel::Receiver<accesskit::ActionRequest>,
    theme_watch: &'a Option<crate::app::theme_file::ThemeWatch>,
    frame_state: &'a mut super::FrameState,
    input_error: Option<Error>,
}

pub(super) fn frame_once(ctx: FrameCtx<'_>) -> AppResult<()> {
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
    let dt = frame_state.animation_clock.elapsed(Instant::now());
    let env = RefCell::new(FrameEnv {
        config,
        shell,
        window_handle,
        renderer,
        surface,
        viewport,
        exit_requested,
        async_rt,
        a11y_action_rx,
        theme_watch,
        frame_state,
        input_error: None,
    });
    let viewport_size = *env.borrow().viewport;
    let out = host.frame(router, viewport_size, dt, frame_hooks(&env));
    let env = env.into_inner();
    if let Some(error) = env.input_error {
        return Err(error);
    }
    if out.exit {
        *env.exit_requested = true;
    }
    crate::profile::tick_frame();
    Ok(())
}

fn frame_hooks<'e, 's: 'e>(env: &'e RefCell<FrameEnv<'s>>) -> FrameHooks<'e> {
    FrameHooks {
        input: Some(Box::new(|host, router| {
            input_phase(&mut env.borrow_mut(), host, router);
        })),
        after_layout: Some(Box::new(|host, router| {
            let env = &mut *env.borrow_mut();
            if let Some((cx, layout, _)) = host.tree() {
                sync_ime(env.shell, env.window_handle, cx, layout, router);
            }
        })),
        paint: Some(Box::new(|host, router, info| {
            paint_phase(&mut env.borrow_mut(), host, router, info);
        })),
    }
}

fn input_phase(env: &mut FrameEnv<'_>, host: &mut Host, router: &mut EventRouter) {
    let _effect = crate::profile::span("effect");
    let _ = env.async_rt.pump();
    watch_theme(host, env.theme_watch.as_ref());
    apply_appearance(host);
    install_shell_features(env.shell, env.window_handle);
    let surface_result = ensure_surface(EnsureSurfaceCtx {
        shell: env.shell,
        window_handle: env.window_handle,
        cx: host.cx(),
        renderer: env.renderer,
        surface: env.surface,
        viewport: env.viewport,
        user_zoom: env.config.user_zoom,
    });
    if let Err(err) = surface_result {
        env.input_error = Some(err);
        return;
    }
    pump_shell_features();
    if let Some((cx, layout, _)) = host.tree() {
        drain_events(DrainCtx {
            shell: env.shell,
            window_handle: env.window_handle,
            cx,
            layout,
            router,
            surface: env.surface,
            renderer: env.renderer,
            viewport: env.viewport,
            config: env.config,
            exit_requested: env.exit_requested,
        });
        event_wiring::drain_a11y_actions(env.a11y_action_rx, cx, router, layout);
    }
}

fn watch_theme(host: &mut Host, watch: Option<&crate::app::theme_file::ThemeWatch>) {
    if let Some(watch) = watch
        && let Some(root) = host.root()
        && crate::app::theme_file::drain(host.cx(), root, watch)
    {
        Runtime::mark_frame_dirty();
    }
}

fn apply_appearance(host: &mut Host) {
    if let Some(theme) = crate::app::appearance::take()
        && let Some(root) = host.root()
        && let Some(node) = host.cx().get_mut(root)
    {
        node.attach(theme);
        Runtime::mark_frame_dirty();
    }
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "menu/tray builds add non-const bodies"
)]
fn install_shell_features(shell: &WinitShell, window_handle: WinitWindowHandle) {
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
    #[cfg(not(any(feature = "menu", feature = "tray")))]
    let _ = (shell, window_handle);
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "menu/tray builds add non-const bodies"
)]
fn pump_shell_features() {
    #[cfg(feature = "menu")]
    crate::app::menu::pump();
    #[cfg(feature = "tray")]
    crate::app::tray::pump();
}

fn paint_phase(env: &mut FrameEnv<'_>, host: &mut Host, router: &EventRouter, info: &FrameInfo) {
    let _paint = crate::profile::span("paint");
    let force_paint = env.shell.take_force_paint();
    let paint_needed = force_paint || info.dirty;
    let allow_replay = force_paint && !info.dirty && !info.animating;
    if allow_replay {
        crate::profile::interact("expose");
    }
    if paint_needed && let Some((cx, _, root)) = host.tree() {
        env.frame_state.scene_cache.paint(PaintRequest {
            cx,
            root,
            renderer: env.renderer,
            surface: env.surface,
            viewport: *env.viewport,
            allow_replay,
        });
    }
    let _a11y = crate::profile::span("a11y");
    if let Some((cx, layout, _)) = host.tree() {
        event_wiring::publish_a11y(env.shell, cx, router, layout, paint_needed);
    }
}

#[cfg(test)]
pub(super) fn begin_effect(async_rt: &mut AsyncRuntime) {
    Runtime::begin_effect();
    let _drained = async_rt.pump();
}

#[cfg(test)]
pub(super) fn run_layout_if_dirty(dirty: bool, layout_pass: impl FnOnce()) -> bool {
    if dirty {
        layout_pass();
    }
    dirty
}
