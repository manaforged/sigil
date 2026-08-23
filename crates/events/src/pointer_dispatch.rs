//! Pointer-event dispatch helpers.
//!
//! Split out of [`crate::router`] so that file stays under the 400-line
//! hygiene cap and so that hover-path maintenance can cache absolute
//! origins as it walks — rebuilding them from `Cx` per node was an
//! O(depth²) hot spot on every hover transition (see
//! `route_hover_transition_depth_20` bench).

#![allow(clippy::redundant_pub_crate)]

use sigil_hal::{
    LogicalSize, ModifiersState, MouseButton, MouseButtons, Point, PointerEvent, PointerEventInit,
    PointerKind, PointerPhase,
};
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::SmallVec;

use crate::EventHandlers;
use crate::hit_test::NodePath;

pub(crate) type PointerTargets = SmallVec<[PointerTarget; 64]>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerTarget {
    pub(crate) id: NodeId,
    pub(crate) origin: Point,
    /// Border-box extent, handed to handlers as `PointerEvent::target_size`
    /// so position->ratio math divides by the real width instead of a
    /// compile-time default.
    pub(crate) size: LogicalSize,
}

/// Dispatch along the hit path (capture pass root→leaf, then bubble
/// leaf→root). Returns the first node whose handler requested pointer
/// capture, so the router can capture the REQUESTER rather than the hit
/// leaf: a drag source wrapping child content (table row over text
/// cells) must keep receiving move/up itself.
pub(crate) fn dispatch_pointer_path(
    cx: &Cx,
    targets: &[PointerTarget],
    event: &mut PointerEvent,
    global_position: Point,
) -> Option<NodeId> {
    let mut capture_requester = None;
    for target in targets.iter().copied() {
        if invoke_pointer(cx, target, event, true, global_position) && capture_requester.is_none() {
            capture_requester = Some(target.id);
        }
        if event.propagation_stopped() {
            return capture_requester;
        }
    }
    for target in targets.iter().rev().copied() {
        if invoke_pointer(cx, target, event, false, global_position) && capture_requester.is_none()
        {
            capture_requester = Some(target.id);
        }
        if event.propagation_stopped() {
            return capture_requester;
        }
    }
    capture_requester
}

pub(crate) fn dispatch_pointer_target(
    cx: &Cx,
    target: PointerTarget,
    event: &mut PointerEvent,
    global_position: Point,
) {
    invoke_pointer(cx, target, event, false, global_position);
}

pub(crate) fn dispatch_synthetic_target(
    cx: &Cx,
    target: PointerTarget,
    source: &PointerEvent,
    phase: PointerPhase,
) {
    let mut event = source.clone();
    event.phase = phase;
    dispatch_pointer_target(cx, target, &mut event, source.position);
}

pub(crate) const fn synthetic_leave_source() -> PointerEvent {
    PointerEvent::new(PointerEventInit {
        position: Point::new(0.0, 0.0),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Leave,
    })
}

pub(crate) fn pointer_targets(layout: &LayoutEngine, path: &NodePath) -> PointerTargets {
    let mut targets = PointerTargets::new();
    for id in path.iter().copied() {
        let rect = layout.result(id).map(|result| result.rect());
        let origin = rect.map_or(Point::new(0.0, 0.0), |r| r.origin);
        let size = rect.map_or(LogicalSize::new(0.0, 0.0), |r| {
            LogicalSize::new(r.width, r.height)
        });
        targets.push(PointerTarget { id, origin, size });
    }
    targets
}

/// Synthesize a click on `target`, dispatched directly to that node
/// rather than via hit-testing a screen position (DR-050 §4,
/// `EventRouter::invoke_node`).
///
/// AT-initiated `accesskit::Action::Click` requests name a node id, not
/// a pointer coordinate. The Enter → Down → Up → Leave sequence mirrors
/// what a real mouse click produces so `InteractionState::release_pointer`'s
/// hover+press gate (every button/checkbox/etc. widget's activation
/// guard) sees the same state a physical click would — a bare Down+Up
/// leaves `hovered` false and silently no-ops the activation.
pub(crate) fn invoke_node(cx: &Cx, layout: &LayoutEngine, target: NodeId) {
    let target = pointer_target_for_node(cx, layout, target);
    for phase in [
        PointerPhase::Enter,
        PointerPhase::Down,
        PointerPhase::Up,
        PointerPhase::Leave,
    ] {
        let button =
            matches!(phase, PointerPhase::Down | PointerPhase::Up).then_some(MouseButton::Left);
        let mut event = PointerEvent::new(PointerEventInit {
            position: target.origin,
            delta: Point::new(0.0, 0.0),
            kind: PointerKind::Mouse,
            button,
            buttons: MouseButtons::empty(),
            modifiers: ModifiersState::empty(),
            phase,
        });
        dispatch_pointer_target(cx, target, &mut event, target.origin);
    }
}

pub(crate) fn pointer_target_for_node(
    cx: &Cx,
    layout: &LayoutEngine,
    target: NodeId,
) -> PointerTarget {
    pointer_targets(layout, &path_to_node(cx, target))
        .last()
        .copied()
        .unwrap_or(PointerTarget {
            id: target,
            origin: Point::new(0.0, 0.0),
            size: LogicalSize::new(0.0, 0.0),
        })
}

/// Invoke the target's handlers for this phase. Returns true when the
/// target's handlers newly requested pointer capture during this invoke.
fn invoke_pointer(
    cx: &Cx,
    target: PointerTarget,
    event: &mut PointerEvent,
    capture: bool,
    global_position: Point,
) -> bool {
    let Some(handlers) = cx
        .get(target.id)
        .and_then(|node| node.get_extension::<EventHandlers>())
    else {
        return false;
    };
    let previous_position = event.position;
    let previous_size = event.target_size;
    event.position = Point::new(
        global_position.x - target.origin.x,
        global_position.y - target.origin.y,
    );
    event.target_size = target.size;
    let already_requested = event.capture_requested();
    handlers.invoke_pointer(event.phase, capture, event);
    let newly_requested = !already_requested && event.capture_requested();
    event.position = previous_position;
    event.target_size = previous_size;
    newly_requested
}

fn path_to_node(cx: &Cx, target: NodeId) -> NodePath {
    let mut reversed = NodePath::new();
    let mut current = Some(target);
    while let Some(id) = current {
        reversed.push(id);
        current = cx.parent(id);
    }
    reversed.reverse();
    reversed
}
