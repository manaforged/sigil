use std::cell::RefCell;
use web_time::Instant;

use crate::app::embed::Host;
use crate::app::error::{AppResult, Error};
use crate::app::{FrameHooks, FrameInfo};
use crate::events::EventRouter;
use crate::reactive::Runtime;
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use super::FrameCtx;
use super::backend::Backend;
use super::event_wiring::{
    self, DrainCtx, EnsureSurfaceCtx, drain_events, ensure_surface, sync_ime,
};
#[cfg(any(feature = "menu", feature = "media"))]
use super::menu_hwnd;
use super::record::{SignalWrite, WindowRecord, WindowView};
use super::scene_cache::PaintRequest;
#[cfg(any(
    feature = "menu",
    feature = "tray",
    feature = "media",
    feature = "hotkey"
))]
use super::wake_fn;

struct FrameEnv<'a> {
    shell: &'a mut WinitShell,
    view: &'a mut WindowView,
    renderer: &'a mut Backend,
    exit_requested: &'a mut bool,
    async_rt: &'a mut AsyncRuntime,
    forced: bool,
    signal_writes: Vec<SignalWrite>,
    input_error: Option<Error>,
}

pub(super) fn frame_once(ctx: FrameCtx<'_>) -> AppResult<()> {
    let FrameCtx {
        shell,
        record,
        renderer,
        exit_requested,
        async_rt,
        clock,
        signal_writes,
    } = ctx;
    let WindowRecord { host, router, view } = record;

    crate::profile::begin_frame(renderer.status());
    let dt = clock.elapsed(Instant::now());
    let forced = std::mem::take(&mut view.repaint);
    view.painted = false;
    let viewport_size = view.viewport;
    let env = RefCell::new(FrameEnv {
        shell,
        view,
        renderer,
        exit_requested,
        async_rt,
        forced,
        signal_writes,
        input_error: None,
    });
    let out = host.frame(router, viewport_size, dt, frame_hooks(&env));
    let env = env.into_inner();
    if let Some(error) = env.input_error {
        return Err(error);
    }
    if out.exit {
        *env.exit_requested = true;
    }
    if forced {
        env.view.painted = false;
    }
    crate::profile::tick_frame();
    Ok(())
}

fn frame_hooks<'e, 's: 'e>(env: &'e RefCell<FrameEnv<'s>>) -> FrameHooks<'e> {
    FrameHooks {
        input: Some(Box::new(|host, router| {
            input_phase(&mut env.borrow_mut(), host, router)
        })),
        after_layout: Some(Box::new(|host, router| {
            let env = &mut *env.borrow_mut();
            if let Some((cx, layout, _)) = host.tree() {
                sync_ime(env.shell, env.view.handle, cx, layout, router);
            }
        })),
        paint: Some(Box::new(|host, router, info| {
            paint_phase(&mut env.borrow_mut(), host, router, info);
        })),
    }
}

fn input_phase(
    env: &mut FrameEnv<'_>,
    host: &mut Host,
    router: &mut EventRouter,
) -> Option<(f32, f32)> {
    let _effect = crate::profile::span(crate::profile::Phase::Effect);
    for (signal, value) in std::mem::take(&mut env.signal_writes) {
        signal.set(value);
    }
    if env.forced {
        Runtime::mark_frame_dirty();
    }
    let _ = env.async_rt.pump();
    install_shell_features(env.shell, env.view.handle, &env.view.config.title);
    let surface_result = ensure_surface(EnsureSurfaceCtx {
        shell: env.shell,
        window_handle: env.view.handle,
        cx: host.cx(),
        renderer: env.renderer,
        surface: &mut env.view.surface,
        viewport: &mut env.view.viewport,
        user_zoom: env.view.config.user_zoom,
    });
    if let Err(err) = surface_result {
        env.input_error = Some(err);
        return None;
    }
    pump_shell_features();
    let actions = env.shell.a11y_action_receiver(env.view.handle);
    if let Some((cx, layout, _)) = host.tree() {
        drain_events(DrainCtx {
            view: env.view,
            cx,
            layout,
            router,
            renderer: env.renderer,
            exit_requested: env.exit_requested,
        });
        if let Some(actions) = &actions {
            event_wiring::drain_a11y_actions(actions, cx, router, layout);
        }
    }
    Some(env.view.viewport)
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "menu/tray builds add non-const bodies"
)]
fn install_shell_features(shell: &WinitShell, window_handle: WinitWindowHandle, title: &str) {
    #[cfg(feature = "menu")]
    if crate::app::menu::has_pending()
        && let Err(error) =
            crate::app::menu::install(menu_hwnd(shell, window_handle), wake_fn(shell))
    {
        tracing::error!("sigil-app: cannot install native menu: {error}");
    }
    #[cfg(feature = "tray")]
    if crate::app::tray::has_pending() {
        crate::app::tray::install(wake_fn(shell));
    }
    #[cfg(feature = "hotkey")]
    crate::app::hotkey::install(wake_fn(shell));
    #[cfg(feature = "media")]
    if !crate::app::media::is_installed()
        && let Err(error) =
            crate::app::media::install(title, menu_hwnd(shell, window_handle), wake_fn(shell))
    {
        tracing::warn!("sigil-app: cannot install media controls: {error}");
    }
    #[cfg(not(any(
        feature = "menu",
        feature = "tray",
        feature = "media",
        feature = "hotkey"
    )))]
    let _ = shell;
    #[cfg(not(any(feature = "menu", feature = "media")))]
    let _ = window_handle;
    #[cfg(not(feature = "media"))]
    let _ = title;
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
    #[cfg(feature = "hotkey")]
    crate::app::hotkey::pump();
    #[cfg(feature = "media")]
    crate::app::media::pump();
}

fn paint_phase(env: &mut FrameEnv<'_>, host: &mut Host, router: &EventRouter, info: &FrameInfo) {
    let _paint = crate::profile::span(crate::profile::Phase::Paint);
    let force_paint = env.shell.take_force_paint();
    let paint_needed = !env.view.hidden && (force_paint || info.dirty);
    let allow_replay = force_paint && !info.dirty && !info.animating;
    if allow_replay {
        crate::profile::interact(crate::profile::Cause::Expose);
    }
    env.view.painted = info.dirty;
    if paint_needed && let Some((cx, _, root)) = host.tree() {
        env.view.scene_cache.paint(PaintRequest {
            cx,
            root,
            renderer: env.renderer,
            surface: &mut env.view.surface,
            viewport: env.view.viewport,
            allow_replay,
        });
    }
    let _a11y = crate::profile::span(crate::profile::Phase::A11y);
    if let Some((cx, layout, _)) = host.tree() {
        event_wiring::publish_a11y(
            env.shell,
            env.view.handle,
            (cx, router, layout),
            paint_needed,
        );
    }
}
