//! Event-drain and window-event helpers for the launch loop.

use events::EventRouter;
use hal::{Key, KeyPhase, PhysicalSize, Scale, Shell as _, ShellEvent, WindowEvent};
use kernel::Cx;
use layout::LayoutEngine;
use reactive::Runtime;
use renderer::{WgpuRenderer, WgpuSurface};
use shell::{WinitShell, WinitWindowHandle};

use crate::config::AppConfig;
use hal::Renderer as _;
use renderer::into_window_handle;
use std::sync::Arc;

use super::{
    clamp_physical_size, logical_size_from_viewport, recompute_layout, resize_surface,
    scale_from_f64, set_viewport,
};

pub(super) struct DrainCtx<'a> {
    pub(super) shell: &'a mut WinitShell,
    pub(super) cx: &'a mut Cx,
    pub(super) root: kernel::NodeId,
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
    pub(super) root: kernel::NodeId,
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
        Ok(new_surface) => {
            *surface = Some(new_surface);
            // First surface (or a re-created one) must paint — it writes no signal.
            Runtime::mark_frame_dirty();
        }
        Err(err) => tracing::warn!("sigil-app: create_surface failed: {err}"),
    }
}

/// End-of-Layout IME sync (DR-051 §Decision-4): push the focused node's
/// IME-allowed state and, when it published a caret anchor this frame,
/// forward the resolved rect to the OS candidate window. Called after
/// layout recomputes so the anchor's absolute origin is current — see
/// DR-051's "why end-of-Layout, not Effect" reasoning.
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
    if *exit_requested {
        shell.request_exit();
    }
}

/// W5-A11Y (DR-050 §1, §8) — project the kernel a11y tree and publish it
/// through a fresh [`a11y::AccessKitBridge`] over every live window's
/// adapter (`shell.publish_a11y_update` broadcasts).
///
/// `dirty` is the SAME whole-frame dirty decision the paint gate uses
/// (`shell.take_force_paint() || Runtime::frame_dirty()`), computed once
/// by the caller — RC-focus adversarial finding 1: this must run AFTER
/// `Runtime::begin_layout()`'s `recompute_layout`, not before, or the
/// published bounds are one frame stale (the moved/resized node's OLD
/// rect) and the corrective republish never happens on a subsequent
/// clean frame (the dirty gate suppresses it). Folding `take_force_paint`
/// into the gate matters on its own: a resize changes bounds without
/// any reactive write, so the a11y tree needs republishing on exactly
/// the same frames the paint gate repaints on.
///
/// Callers run `drain_a11y_actions` first, then this — RC-focus finding
/// (round 2): draining actions BEFORE the publish means `set_focus`'s
/// effect is visible in THIS frame's tree (focus is a plain field read,
/// no layout dependency), while `invoke_node`'s downstream widget-state
/// changes settle on the NEXT frame's Effect→Layout cycle (one-frame
/// action latency — standard, not a bug: the same latency a real click
/// has between input and the next paint).
///
/// The bridge's `action_rx` half is unused here (inbound actions are
/// already drained earlier this frame by `drain_a11y_actions`) —
/// cloning the receiver is a cheap `Arc` bump, not a channel
/// allocation, so constructing a second bridge purely for its `publish`
/// half costs nothing meaningful.
pub(super) fn publish_a11y(
    shell: &mut WinitShell,
    cx: &Cx,
    router: &EventRouter,
    layout: &LayoutEngine,
    dirty: bool,
) {
    if !dirty {
        return;
    }
    let update = a11y::tree(cx, router.focus(), &|id| {
        layout.result(id).map(|result| result.rect())
    });
    let action_rx = shell.a11y_action_receiver();
    let mut bridge = a11y::AccessKitBridge::new(action_rx, |tree_update| {
        shell.publish_a11y_update(&tree_update);
    });
    let _ = hal::A11yBridge::publish(&mut bridge, update);
}

/// W5-A11Y (DR-050 §4) — route AT-initiated actions through the same
/// `EventRouter` surface pointer/keyboard input use, validated through
/// the exact same interactivity gates that surface applies to real
/// input (RC-focus adversarial finding 3): `Focus` requires
/// `EventRouter::is_focus_target` (focusable, laid out, and inside the
/// active focus scope — reuses the a4972ca tab-ring semantics); `Click`
/// requires `EventRouter::is_click_target` (laid out, hit-test-enabled,
/// and inside the active focus scope). A platform request naming a
/// hidden, non-focusable, or behind-a-modal-scope node is silently
/// rejected — `invoke_node`'s direct dispatch bypasses hit-testing
/// entirely, so nothing else would otherwise stop it reaching a node a
/// real click could never touch. Every other `accesskit::Action`
/// variant (Increment/Decrement/SetValue/...) is intentionally a no-op
/// here — the Wave-5 design brief (item 7) names the per-widget
/// state-signal wiring those actions need as separate, wider
/// Wave-5-widget-catalogue work, not part of this slice.
///
/// Called AFTER `recompute_layout` (RC-focus finding, round 2) — the
/// caller used to run this from inside `drain_events`, in the Effect
/// phase, BEFORE layout recomputed. An Effect-phase mutation that hides
/// the queued action's target this frame (e.g. a modal-dismiss handler
/// firing earlier in the same `ShellEvent` batch) would validate against
/// the PREVIOUS frame's stale layout/scope state, where the target still
/// looked laid out — the action would fire on a now-blocked control.
/// `is_focus_target`/`is_click_target` both read `layout.result(...)`
/// directly, so the fix is purely about WHEN this runs, not the
/// validators themselves.
///
/// Drains through a throwaway [`a11y::AccessKitBridge`] (no-op publish
/// sink — nothing is published from this call) rather than the raw
/// channel directly, so the `accesskit::NodeId → kernel::NodeId`
/// reverse mapping lives in exactly one place
/// (`AccessKitBridge::take_action`).
pub(super) fn drain_a11y_actions(
    action_rx: &crossbeam_channel::Receiver<accesskit::ActionRequest>,
    cx: &mut Cx,
    router: &mut EventRouter,
    layout: &LayoutEngine,
) {
    let mut bridge = a11y::AccessKitBridge::new(action_rx.clone(), |_: accesskit::TreeUpdate| {});
    while let Some((target, request)) = hal::A11yBridge::take_action(&mut bridge) {
        match request.action {
            accesskit::Action::Focus if router.is_focus_target(cx, layout, target) => {
                router.set_focus(cx, Some(target));
            }
            accesskit::Action::Click if router.is_click_target(cx, layout, target) => {
                events::invoke_node(cx, layout, target);
            }
            _ => {}
        }
    }
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    root: kernel::NodeId,
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
    // Resize repaints at the new size — it writes no signal.
    Runtime::mark_frame_dirty();
}

fn handle_scale_changed(ctx: &mut WindowEventCtx<'_>, scale: Scale) {
    ctx.cx.set_scale(scale);
    let logical = logical_size_from_viewport(*ctx.viewport);
    resize_surface(ctx.renderer, ctx.surface, scale.to_physical(logical));
    recompute_layout(ctx.cx, ctx.root, ctx.layout, scale, logical);
    // DPI change repaints (text re-rasterizes at the new scale) — no signal.
    Runtime::mark_frame_dirty();
}
