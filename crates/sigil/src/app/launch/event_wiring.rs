use crate::events::EventRouter;
use crate::events::{Event, WindowEvent};
use crate::geometry::{PhysicalSize, Scale};
use crate::hal::Shell as _;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;
use crate::profile::Cause;
use crate::reactive::Runtime;
use crate::shell::{WinitShell, WinitWindowHandle};

use crate::app::config::QuitKeys;
use crate::error::AppResult;
use std::sync::Arc;

use super::backend::{Backend, RenderSurface};
use super::record::WindowView;
use super::{clamp_physical_size, resize_surface, scale_from_f64};
use crate::geometry::Size;

pub(super) struct DrainCtx<'a> {
    pub(super) view: &'a mut WindowView,
    pub(super) cx: &'a mut Cx,
    pub(super) layout: &'a LayoutEngine,
    pub(super) router: &'a mut EventRouter,
    pub(super) renderer: &'a mut Backend,
}

pub(super) struct EnsureSurfaceCtx<'a> {
    pub(super) shell: &'a WinitShell,
    pub(super) window_handle: WinitWindowHandle,
    pub(super) cx: &'a mut Cx,
    pub(super) renderer: &'a mut Backend,
    pub(super) surface: &'a mut Option<RenderSurface>,
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
    #[cfg(not(target_arch = "wasm32"))]
    if renderer.recover(surface)? {
        Runtime::mark_frame_dirty();
    }
    if surface.is_some() {
        return Ok(());
    }
    let Some(window) = shell.window_for(window_handle) else {
        return Ok(());
    };

    let scale = scale_from_f64(window.scale_factor() * f64::from(user_zoom));
    cx.set_scale(scale.factor());
    let inner = window.inner_size();
    if inner.width > 0 && inner.height > 0 {
        let physical = clamp_physical_size(PhysicalSize::new(inner.width, inner.height));
        let logical = scale.to_logical(physical);
        *viewport = (logical.width, logical.height);
    }
    let logical = Size::new(viewport.0, viewport.1);

    #[cfg(not(target_arch = "wasm32"))]
    renderer.set_wake(super::wake_fn(shell));
    let new_surface = renderer.create_surface(Arc::clone(window), scale.to_physical(logical))?;
    tracing::info!(
        "sigil-app: rendering with the {} backend",
        renderer.status()
    );
    *surface = Some(new_surface);
    Runtime::mark_frame_dirty();
    Ok(())
}

pub(super) fn sync_cursor(
    shell: &mut WinitShell,
    window_handle: WinitWindowHandle,
    cx: &Cx,
    layout: &LayoutEngine,
    router: &mut EventRouter,
) {
    if let Err(err) = shell.set_cursor(&window_handle, router.cursor(cx, layout)) {
        tracing::warn!(target: "sigil::shell", error = %err, "cursor update failed");
    }
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
    } = ctx;
    router.set_quit(QuitKeys {
        ctrl_c: false,
        ..view.config.quit
    });
    for event in core::mem::take(&mut view.inbox) {
        crate::profile::note_input(&event);
        if let Event::Window(window_event) = &event {
            let mut window_ctx = WindowEventCtx {
                cx,
                renderer,
                surface: &mut view.surface,
                viewport: &mut view.viewport,
            };
            window_ctx.handle(window_event);
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
    bridge.publish(update);
}

pub(super) fn drain_a11y_actions(
    action_rx: &std::rc::Rc<std::sync::mpsc::Receiver<accesskit::ActionRequest>>,
    cx: &mut Cx,
    router: &mut EventRouter,
    layout: &LayoutEngine,
) {
    let bridge = crate::a11y::AccessKitBridge::new(
        std::rc::Rc::clone(action_rx),
        |_: accesskit::TreeUpdate| {},
    );
    while let Some((target, request)) = bridge.take_action() {
        crate::profile::interact(Cause::A11y);
        crate::a11y::actions::apply(cx, router, layout, target, &request);
    }
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    renderer: &'a mut Backend,
    surface: &'a mut Option<RenderSurface>,
    viewport: &'a mut (f32, f32),
}

impl WindowEventCtx<'_> {
    fn handle(&mut self, event: &WindowEvent) {
        match *event {
            WindowEvent::Resized(physical) => {
                resize_surface(self.renderer, self.surface, physical);
                let logical = Scale::new(self.cx.scale()).to_logical(clamp_physical_size(physical));
                *self.viewport = (logical.width, logical.height);
            }
            WindowEvent::ScaleFactorChanged(scale) => {
                self.cx.set_scale(scale.factor());
                let logical = Size::new(self.viewport.0, self.viewport.1);
                resize_surface(self.renderer, self.surface, scale.to_physical(logical));
            }
            WindowEvent::BackdropChanged(backdrop) => {
                crate::app::backdrop::set_system_backdrop(backdrop);
            }
            WindowEvent::Focused(true) => crate::app::platform_prefs::refresh(),
            WindowEvent::CloseRequested
            | WindowEvent::Destroyed
            | WindowEvent::Focused(false)
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
