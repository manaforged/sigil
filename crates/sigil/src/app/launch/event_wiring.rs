use crate::events::EventRouter;
use crate::hal::{
    Key, KeyPhase, PhysicalSize, PointerPhase, Scale, Shell as _, ShellEvent, WindowEvent,
};
use crate::kernel::Cx;
use crate::layout::LayoutEngine;
use crate::reactive::Runtime;
use crate::shell::{WinitShell, WinitWindowHandle};

use crate::app::config::AppConfig;
use std::sync::Arc;

use super::backend::{Backend, Surface};
use super::{
    clamp_physical_size, logical_size_from_viewport, resize_surface, scale_from_f64, set_viewport,
};

pub(super) struct DrainCtx<'a> {
    pub(super) shell: &'a mut WinitShell,
    pub(super) window_handle: WinitWindowHandle,
    pub(super) cx: &'a mut Cx,
    pub(super) layout: &'a LayoutEngine,
    pub(super) router: &'a mut EventRouter,
    pub(super) surface: &'a mut Option<Surface>,
    pub(super) renderer: &'a mut Backend,
    pub(super) viewport: &'a mut (f32, f32),
    pub(super) config: &'a AppConfig,
    pub(super) exit_requested: &'a mut bool,
}

pub(super) struct EnsureSurfaceCtx<'a> {
    pub(super) shell: &'a WinitShell,
    pub(super) window_handle: WinitWindowHandle,
    pub(super) cx: &'a mut Cx,
    pub(super) renderer: &'a mut Backend,
    pub(super) surface: &'a mut Option<Surface>,
    pub(super) viewport: &'a (f32, f32),
    pub(super) user_zoom: f32,
}

pub(super) fn ensure_surface(ctx: EnsureSurfaceCtx<'_>) {
    let EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        renderer,
        surface,
        viewport,
        user_zoom,
    } = ctx;
    if surface.is_some() {
        return;
    }
    let Some(window) = shell.window_for(window_handle) else {
        return;
    };

    let scale = scale_from_f64(window.scale_factor() * f64::from(user_zoom));
    cx.set_scale(scale);
    let logical = logical_size_from_viewport(*viewport);

    match renderer.create_surface(Arc::clone(window), scale.to_physical(logical)) {
        Ok(new_surface) => {
            tracing::info!("sigil-app: rendering with the {} backend", renderer.name());
            *surface = Some(new_surface);
            Runtime::mark_frame_dirty();
        }
        Err(err) => tracing::warn!("sigil-app: create_surface failed: {err}"),
    }
}

pub(super) fn sync_ime(
    shell: &mut WinitShell,
    window_handle: WinitWindowHandle,
    cx: &Cx,
    layout: &LayoutEngine,
    router: &EventRouter,
) {
    let _ = shell.set_ime_allowed(&window_handle, router.focus_wants_ime(cx));
    if let Some(rect) = router.ime_caret_rect(cx, layout) {
        let _ = shell.set_ime_cursor_area(&window_handle, rect);
    }
}

pub(super) fn drain_events(ctx: DrainCtx<'_>) {
    let DrainCtx {
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
    } = ctx;
    for event in shell.drain_events() {
        note_input(&event);
        if let ShellEvent::Window(window_event) = &event {
            let mut window_ctx = WindowEventCtx {
                cx,
                renderer,
                surface,
                viewport,
                exit_requested,
            };
            handle_window_event(&mut window_ctx, window_event);
        }
        match &event {
            ShellEvent::Key(key_event)
                if config.esc_exits
                    && key_event.key == Key::Escape.into()
                    && key_event.phase == KeyPhase::Down =>
            {
                *exit_requested = true;
            }
            _ => {}
        }
        router.route(cx, layout, event);
    }
    for command in crate::events::take_window_commands() {
        let _ = shell.window_command(&window_handle, command);
    }
    if *exit_requested {
        shell.request_exit();
    }
}

fn note_input(event: &ShellEvent) {
    if !crate::profile::enabled() {
        return;
    }
    match event {
        ShellEvent::Pointer(pointer) => match pointer.phase {
            PointerPhase::Down | PointerPhase::Up => crate::profile::interact("pointer"),
            PointerPhase::Move => crate::profile::interact("hover"),
            PointerPhase::Enter | PointerPhase::Leave | PointerPhase::Cancel => {}
        },
        ShellEvent::Key(_) => crate::profile::interact("key"),
        ShellEvent::Scroll(_) => crate::profile::interact("scroll"),
        ShellEvent::Ime(_) => crate::profile::interact("ime"),
        ShellEvent::Window(WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged(_)) => {
            crate::profile::interact("resize");
        }
        ShellEvent::Window(_) | ShellEvent::FileDrop(_) => {}
    }
}

pub(super) fn publish_a11y(
    shell: &mut WinitShell,
    cx: &Cx,
    router: &EventRouter,
    layout: &LayoutEngine,
    dirty: bool,
) {
    let fresh = shell.take_a11y_activation();
    if !shell.a11y_active() || !(dirty || fresh) {
        return;
    }
    let update = crate::a11y::tree(cx, router.focus(), &|id| {
        layout.result(id).map(|result| result.rect())
    });
    let action_rx = shell.a11y_action_receiver();
    let mut bridge = crate::a11y::AccessKitBridge::new(action_rx, |tree_update| {
        shell.publish_a11y_update(&tree_update);
    });
    let _ = crate::hal::A11yBridge::publish(&mut bridge, update);
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
        crate::profile::interact("a11y");
        router.focus_if_needed(cx, layout);
        match request.action {
            accesskit::Action::Focus if router.is_focus_target(cx, layout, target) => {
                router.set_focus(cx, Some(target));
            }
            accesskit::Action::Click if router.is_click_target(cx, layout, target) => {
                crate::events::invoke_node(cx, layout, target);
            }
            accesskit::Action::Increment if router.is_focus_target(cx, layout, target) => {
                adjust_value(cx, target, true);
            }
            accesskit::Action::Decrement if router.is_focus_target(cx, layout, target) => {
                adjust_value(cx, target, false);
            }
            _ => {}
        }
    }
}

fn adjust_value(cx: &Cx, target: crate::kernel::NodeId, increase: bool) {
    let Some(node) = cx.get(target) else {
        return;
    };
    let (Some(value), Some(min), Some(max)) = (
        node.a11y.value_now,
        node.a11y.value_min,
        node.a11y.value_max,
    ) else {
        return;
    };
    let step = node.a11y.step.unwrap_or(1.0);
    let next = if increase {
        value.peek() + step
    } else {
        value.peek() - step
    };
    value.set(next.clamp(min, max));
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    renderer: &'a mut Backend,
    surface: &'a mut Option<Surface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
}

fn handle_window_event(ctx: &mut WindowEventCtx<'_>, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested | WindowEvent::Destroyed => {
            *ctx.exit_requested = true;
        }
        WindowEvent::Resized(size) => handle_resized(ctx, *size),
        WindowEvent::ScaleFactorChanged(scale) => handle_scale_changed(ctx, *scale),
        WindowEvent::Focused(_) | WindowEvent::Occluded(_) => {}
    }
}

fn handle_resized(ctx: &mut WindowEventCtx<'_>, physical: PhysicalSize) {
    resize_surface(ctx.renderer, ctx.surface, physical);
    let scale = ctx.cx.scale();
    let logical = scale.to_logical(clamp_physical_size(physical));
    set_viewport(ctx.viewport, logical);
    Runtime::mark_frame_dirty();
}

fn handle_scale_changed(ctx: &mut WindowEventCtx<'_>, scale: Scale) {
    ctx.cx.set_scale(scale);
    resize_surface(
        ctx.renderer,
        ctx.surface,
        scale.to_physical(super::logical_size_from_viewport(*ctx.viewport)),
    );
    Runtime::mark_frame_dirty();
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
