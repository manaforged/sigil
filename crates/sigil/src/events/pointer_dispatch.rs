#![allow(clippy::redundant_pub_crate)]

use crate::events::{MouseButton, PointerEvent, PointerPhase};
use crate::geometry::{Point, Size};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use smallvec::SmallVec;

use crate::events::hit_test::{NodePath, local_position};
use crate::style::StyleBinding;

pub(crate) type PointerTargets = SmallVec<[PointerTarget; 64]>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerTarget {
    pub(crate) id: NodeId,
    pub(crate) origin: Point,
    pub(crate) size: Size,
    transformed: bool,
}

pub(crate) fn dispatch_pointer_path(
    cx: &Cx,
    targets: &[PointerTarget],
    event: &mut PointerEvent,
    global_position: Point,
) -> Option<NodeId> {
    let down = targets.iter().map(|target| (*target, true));
    let up = targets.iter().rev().map(|target| (*target, false));
    let mut capture_requester = None;
    for (target, capture) in down.chain(up) {
        if invoke_pointer(cx, target, event, capture, global_position)
            && capture_requester.is_none()
        {
            capture_requester = Some(target.id);
        }
        if event.propagation_stopped() {
            break;
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
    mouse_event(Point::new(0.0, 0.0), None, PointerPhase::Leave)
}

const fn mouse_event(
    position: Point,
    button: Option<MouseButton>,
    phase: PointerPhase,
) -> PointerEvent {
    let mut event = PointerEvent::new(phase, position);
    event.button = button;
    event
}

pub(crate) fn pointer_targets(
    cx: &Cx,
    layout: &LayoutEngine,
    path: &NodePath,
    visual_transforms: bool,
) -> PointerTargets {
    let mut targets = PointerTargets::new();
    let mut transformed = false;
    for id in path.iter().copied() {
        if visual_transforms && let Some(node) = cx.get(id) {
            if node.extension::<crate::kernel::TopLayer>().is_some() {
                transformed = false;
            }
            transformed |= node
                .extension::<StyleBinding>()
                .is_some_and(StyleBinding::may_transform);
        }
        let (origin, size) = layout
            .result(id)
            .map(|result| result.rect())
            .map_or((Point::new(0.0, 0.0), Size::new(0.0, 0.0)), |r| {
                (r.origin, Size::new(r.width, r.height))
            });
        targets.push(PointerTarget {
            id,
            origin,
            size,
            transformed,
        });
    }
    targets
}

pub(crate) fn invoke_node(cx: &Cx, layout: &LayoutEngine, target: NodeId, hovered: bool) {
    let target = pointer_target_for_node(cx, layout, target, false);
    let press = [PointerPhase::Down, PointerPhase::Up];
    let visit = [
        PointerPhase::Enter,
        PointerPhase::Down,
        PointerPhase::Up,
        PointerPhase::Leave,
    ];
    let phases: &[PointerPhase] = if hovered { &press } else { &visit };
    for phase in phases.iter().copied() {
        let button =
            matches!(phase, PointerPhase::Down | PointerPhase::Up).then_some(MouseButton::Left);
        let mut event = mouse_event(target.origin, button, phase);
        dispatch_pointer_target(cx, target, &mut event, target.origin);
    }
}

pub(crate) fn pointer_target_for_node(
    cx: &Cx,
    layout: &LayoutEngine,
    target: NodeId,
    visual_transforms: bool,
) -> PointerTarget {
    pointer_targets(cx, layout, &path_to_node(cx, target), visual_transforms)
        .last()
        .copied()
        .unwrap_or(PointerTarget {
            id: target,
            origin: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
            transformed: false,
        })
}

pub(crate) fn local_point(cx: &Cx, target: PointerTarget, global_position: Point) -> Point {
    let local = if target.transformed {
        local_position(cx, target.id, global_position)
    } else {
        Some(global_position)
    };
    local.map_or(Point::new(-1.0, -1.0), |position| {
        Point::new(position.x - target.origin.x, position.y - target.origin.y)
    })
}

fn invoke_pointer(
    cx: &Cx,
    target: PointerTarget,
    event: &mut PointerEvent,
    capture: bool,
    global_position: Point,
) -> bool {
    let previous_position = event.position;
    let previous_size = event.target_size;
    event.position = local_point(cx, target, global_position);
    event.target_size = target.size;
    let already_requested = event.capture_requested();
    let kind = crate::events::handlers::EventKind::pointer(event.phase);
    let _ = crate::events::handlers::fire(
        cx,
        target.id,
        kind,
        crate::events::handlers::Phase::of(capture),
        crate::events::handlers::EventArg::Pointer(event),
    );
    let newly_requested = !already_requested && event.capture_requested();
    event.position = previous_position;
    event.target_size = previous_size;
    newly_requested
}

pub(crate) fn path_to_node(cx: &Cx, target: NodeId) -> NodePath {
    let mut path: NodePath = std::iter::successors(Some(target), |id| cx.parent(*id)).collect();
    path.reverse();
    path
}

/// Counts quick presses on one target, for double and triple clicks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Clicks<K> {
    last: Option<(web_time::Instant, K, Point)>,
    count: u32,
}

impl<K> Default for Clicks<K> {
    fn default() -> Self {
        Self {
            last: None,
            count: 0,
        }
    }
}

impl<K: Copy + PartialEq> Clicks<K> {
    /// Records a press on `target` at `point` and returns its place in the run: 1, 2, 3, ...
    pub(crate) fn press(&mut self, target: K, point: Point) -> u32 {
        let now = web_time::Instant::now();
        let repeat = self.last.is_some_and(|(at, last, from)| {
            last == target
                && crate::events::ClickConfig::current().is_repeat(
                    now.duration_since(at),
                    from,
                    point,
                )
        });
        self.count = if repeat {
            self.count.saturating_add(1)
        } else {
            1
        };
        self.last = Some((now, target, point));
        self.count
    }
}
