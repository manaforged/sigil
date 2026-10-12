use crate::events::EventRouter;
use crate::events::{Key, KeyPhase, ShellEvent, WindowEvent};
use crate::geometry::PhysicalSize;
use crate::hal::Shell as _;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;
use crate::profile::Cause;
use crate::reactive::Runtime;
use crate::shell::{WinitShell, WinitWindowHandle};

use crate::app::error::AppResult;
use std::sync::Arc;

use super::backend::{Backend, Surface};
use super::record::WindowView;
use super::{clamp_physical_size, resize_surface, scale_from_f64};
use crate::geometry::LogicalSize;

pub(super) struct DrainCtx<'a> {
    pub(super) view: &'a mut WindowView,
    pub(super) cx: &'a mut Cx,
    pub(super) layout: &'a LayoutEngine,
    pub(super) router: &'a mut EventRouter,
    pub(super) renderer: &'a mut Backend,
    pub(super) exit_requested: &'a mut bool,
}

pub(super) struct EnsureSurfaceCtx<'a> {
    pub(super) shell: &'a WinitShell,
    pub(super) window_handle: WinitWindowHandle,
    pub(super) cx: &'a mut Cx,
    pub(super) renderer: &'a mut Backend,
    pub(super) surface: &'a mut Option<Surface>,
    pub(super) viewport: &'a mut (f32, f32),
    pub(super) user_zoom: f32,
}

pub(super) fn ensure_surface(ctx: EnsureSurfaceCtx<'_>) -> AppResult<()> {
    let EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        renderer,
        surface,
        viewport,
        user_zoom,
    } = ctx;
    #[cfg(target_arch = "wasm32")]
    if !renderer.ready()? {
        *surface = None;
        return Ok(());
    }
    if surface.is_some() {
        return Ok(());
    }
    let Some(window) = shell.window_for(window_handle) else {
        return Ok(());
    };

    let scale = scale_from_f64(window.scale_factor() * f64::from(user_zoom));
    cx.set_scale(scale);
    let inner = window.inner_size();
    if inner.width > 0 && inner.height > 0 {
        let physical = clamp_physical_size(PhysicalSize::new(inner.width, inner.height));
        let logical = scale.to_logical(physical);
        *viewport = (logical.width, logical.height);
    }
    let logical = LogicalSize::new(viewport.0, viewport.1);

    let new_surface = renderer.create_surface(Arc::clone(window), scale.to_physical(logical))?;
    tracing::info!(
        "sigil-app: rendering with the {} backend",
        renderer.status()
    );
    *surface = Some(new_surface);
    Runtime::mark_frame_dirty();
    Ok(())
}

pub(super) fn sync_ime(
    shell: &mut WinitShell,
    window_handle: WinitWindowHandle,
    cx: &Cx,
    layout: &LayoutEngine,
    router: &EventRouter,
) {
    #[cfg(target_arch = "wasm32")]
    shell.set_tab_edges(router.tab_edges(cx));
    if let Err(err) = shell.set_ime_allowed(&window_handle, router.focus_wants_ime(cx)) {
        tracing::warn!(target: "sigil::ime", error = %err, "IME enable update failed");
    }
    if let Some(rect) = router.ime_caret_rect(cx, layout)
        && let Err(err) = shell.set_ime_cursor_area(&window_handle, rect)
    {
        tracing::warn!(target: "sigil::ime", error = %err, "IME cursor area update failed");
    }
}

pub(super) fn drain_events(ctx: DrainCtx<'_>) {
    let DrainCtx {
        view,
        cx,
        layout,
        router,
        renderer,
        exit_requested,
    } = ctx;
    for event in core::mem::take(&mut view.inbox) {
        crate::profile::note_input(&event);
        if let ShellEvent::Window(window_event) = &event {
            let mut window_ctx = WindowEventCtx {
                cx,
                renderer,
                surface: &mut view.surface,
                viewport: &mut view.viewport,
            };
            window_ctx.handle(window_event);
        }
        if let ShellEvent::Key(key_event) = &event
            && view.config.quit.esc
            && key_event.key == Key::Escape.into()
            && key_event.phase == KeyPhase::Down
        {
            *exit_requested = true;
        }
        router.route(cx, layout, event);
    }
}

pub(super) fn publish_a11y(
    shell: &mut WinitShell,
    handle: WinitWindowHandle,
    tree: (&Cx, &EventRouter, &LayoutEngine),
    dirty: bool,
) {
    let (cx, router, layout) = tree;
    let fresh = shell.take_a11y_activation(handle);
    if !shell.a11y_active(handle) || !(dirty || fresh) {
        return;
    }
    let Some(action_rx) = shell.a11y_action_receiver(handle) else {
        return;
    };
    let update = crate::a11y::tree(cx, router.focus(), &|id| {
        layout
            .result(id)
            .map(|result| crate::events::hit_test::visual_bounds(cx, id, result.rect()))
    });
    let mut bridge = crate::a11y::AccessKitBridge::new(action_rx, |tree_update| {
        shell.publish_a11y_update(handle, tree_update);
    });
    if let Err(err) = crate::hal::A11yBridge::publish(&mut bridge, update) {
        tracing::warn!(target: "sigil::a11y", error = ?err, "accessibility update publish failed");
    }
}

pub(super) fn drain_a11y_actions(
    action_rx: &crossbeam_channel::Receiver<accesskit::ActionRequest>,
    cx: &mut Cx,
    router: &mut EventRouter,
    layout: &LayoutEngine,
) {
    let mut bridge =
        crate::a11y::AccessKitBridge::new(action_rx.clone(), |_: accesskit::TreeUpdate| {});
    while let Some((target, request)) = crate::hal::A11yBridge::take_action(&mut bridge) {
        crate::profile::interact(Cause::A11y);
        crate::a11y::actions::apply(cx, router, layout, target, &request);
    }
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    renderer: &'a mut Backend,
    surface: &'a mut Option<Surface>,
    viewport: &'a mut (f32, f32),
}

impl WindowEventCtx<'_> {
    fn handle(&mut self, event: &WindowEvent) {
        match *event {
            WindowEvent::Resized(physical) => {
                resize_surface(self.renderer, self.surface, physical);
                let logical = self.cx.scale().to_logical(clamp_physical_size(physical));
                *self.viewport = (logical.width, logical.height);
            }
            WindowEvent::ScaleFactorChanged(scale) => {
                self.cx.set_scale(scale);
                let logical = LogicalSize::new(self.viewport.0, self.viewport.1);
                resize_surface(self.renderer, self.surface, scale.to_physical(logical));
            }
            WindowEvent::CloseRequested
            | WindowEvent::Destroyed
            | WindowEvent::Focused(_)
            | WindowEvent::Occluded(_) => return,
        }
        Runtime::mark_frame_dirty();
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "event_wiring_tests.rs"]
mod tests;
