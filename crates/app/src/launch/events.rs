//! Event-drain and window-event helpers for the launch loop.

use sigil_events::EventRouter;
use sigil_hal::{Key, KeyPhase, PhysicalSize, Scale, Shell as _, ShellEvent, WindowEvent};
use sigil_kernel::Cx;
use sigil_layout::LayoutEngine;
use sigil_renderer::{WgpuRenderer, WgpuSurface};
use sigil_shell::{WinitShell, WinitWindowHandle};

use crate::config::AppConfig;
use sigil_hal::Renderer as _;
use sigil_renderer::into_window_handle;
use std::sync::Arc;

use super::{
    clamp_physical_size, logical_size_from_viewport, recompute_layout, resize_surface,
    scale_from_f64, set_viewport,
};

pub(super) struct DrainCtx<'a> {
    pub(super) shell: &'a mut WinitShell,
    pub(super) cx: &'a mut Cx,
    pub(super) root: sigil_kernel::NodeId,
    pub(super) layout: &'a mut LayoutEngine,
    pub(super) router: &'a mut EventRouter,
    pub(super) surface: &'a mut Option<WgpuSurface>,
    pub(super) renderer: &'a mut WgpuRenderer,
    pub(super) viewport: &'a mut (f32, f32),
    pub(super) config: &'a AppConfig,
    pub(super) exit_requested: &'a mut bool,
}

pub(super) struct EnsureSurfaceCtx<'a> {
    pub(super) shell: &'a WinitShell,
    pub(super) window_handle: WinitWindowHandle,
    pub(super) cx: &'a mut Cx,
    pub(super) root: sigil_kernel::NodeId,
    pub(super) layout: &'a mut LayoutEngine,
    pub(super) renderer: &'a mut WgpuRenderer,
    pub(super) surface: &'a mut Option<WgpuSurface>,
    pub(super) viewport: &'a mut (f32, f32),
}

pub(super) fn ensure_surface(ctx: EnsureSurfaceCtx<'_>) {
    let EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        root,
        layout,
        renderer,
        surface,
        viewport,
    } = ctx;
    if surface.is_some() {
        return;
    }
    let Some(window) = shell.window_for(window_handle) else {
        return;
    };

    let scale = scale_from_f64(window.scale_factor());
    cx.set_scale(scale);
    let logical = logical_size_from_viewport(*viewport);
    recompute_layout(cx, root, layout, scale, logical);

    let handle = into_window_handle(Arc::clone(window));
    match renderer.create_surface(handle, scale.to_physical(logical)) {
        Ok(new_surface) => *surface = Some(new_surface),
        Err(err) => tracing::warn!("sigil-app: create_surface failed: {err}"),
    }
}

pub(super) fn drain_events(ctx: DrainCtx<'_>) {
    let DrainCtx {
        shell,
        cx,
        root,
        layout,
        router,
        surface,
        renderer,
        viewport,
        config,
        exit_requested,
    } = ctx;
    for event in shell.drain_events() {
        if let ShellEvent::Window(window_event) = &event {
            let mut window_ctx = WindowEventCtx {
                cx,
                root,
                layout,
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
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    root: sigil_kernel::NodeId,
    layout: &'a mut LayoutEngine,
    renderer: &'a mut WgpuRenderer,
    surface: &'a mut Option<WgpuSurface>,
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
    recompute_layout(ctx.cx, ctx.root, ctx.layout, scale, logical);
}

fn handle_scale_changed(ctx: &mut WindowEventCtx<'_>, scale: Scale) {
    ctx.cx.set_scale(scale);
    let logical = logical_size_from_viewport(*ctx.viewport);
    resize_surface(ctx.renderer, ctx.surface, scale.to_physical(logical));
    recompute_layout(ctx.cx, ctx.root, ctx.layout, scale, logical);
}
