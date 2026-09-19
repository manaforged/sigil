use std::time::Instant;

use crate::app::error::AppResult;
use crate::app::exit::take_exit;
#[cfg(feature = "menu")]
use crate::hal::Shell;
use crate::reactive::{Runtime, tick_intervals};
use crate::tasks::AsyncRuntime;

use super::FrameCtx;
use super::event_wiring::{
    self, DrainCtx, EnsureSurfaceCtx, drain_events, ensure_surface, sync_ime,
};
#[cfg(feature = "menu")]
use super::menu_hwnd;
use super::scene_cache::PaintRequest;

#[allow(
    clippy::too_many_lines,
    reason = "one sequential pass; splitting would scatter the ordering it documents"
)]
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
    })?;
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
    Ok(())
}

pub(super) fn begin_effect(async_rt: &mut AsyncRuntime) {
    Runtime::begin_effect();
    let _drained = async_rt.pump();
}

pub(super) fn run_layout_if_dirty(dirty: bool, layout_pass: impl FnOnce()) -> bool {
    if dirty {
        layout_pass();
    }
    dirty
}
