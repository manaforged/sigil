//! Pointer-event dispatch helpers.
//!
//! Split out of [`crate::router`] so that file stays under the 400-line
//! hygiene cap and so that hover-path maintenance can cache absolute
//! origins as it walks — rebuilding them from `Cx` per node was an
//! O(depth²) hot spot on every hover transition (see
//! `route_hover_transition_depth_20` bench).

#![allow(clippy::redundant_pub_crate)]

use hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEvent, PointerEventInit, PointerKind,
    PointerPhase,
};
use kernel::{Cx, NodeId};
use layout::LayoutEngine;
use smallvec::SmallVec;

use crate::EventHandlers;
use crate::hit_test::NodePath;

pub(crate) type PointerTargets = SmallVec<[PointerTarget; 64]>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerTarget {
    pub(crate) id: NodeId,
    pub(crate) origin: Point,
}

pub(crate) fn dispatch_pointer_path(
    cx: &Cx,
    targets: &[PointerTarget],
    event: &mut PointerEvent,
    global_position: Point,
) {
    for target in targets.iter().copied() {
        invoke_pointer(cx, target, event, true, global_position);
        if event.propagation_stopped() {
            return;
        }
    }
    for target in targets.iter().rev().copied() {
        invoke_pointer(cx, target, event, false, global_position);
        if event.propagation_stopped() {
            return;
        }
    }
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
        let origin = layout
            .result(id)
            .map_or(Point::new(0.0, 0.0), |result| result.rect().origin);
        targets.push(PointerTarget { id, origin });
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
        })
}

fn invoke_pointer(
    cx: &Cx,
    target: PointerTarget,
    event: &mut PointerEvent,
    capture: bool,
    global_position: Point,
) {
    let Some(handlers) = cx
        .get(target.id)
        .and_then(|node| node.get_extension::<EventHandlers>())
    else {
        return;
    };
    let previous_position = event.position;
    event.position = Point::new(
        global_position.x - target.origin.x,
        global_position.y - target.origin.y,
    );
    handlers.invoke_pointer(event.phase, capture, event);
    event.position = previous_position;
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
