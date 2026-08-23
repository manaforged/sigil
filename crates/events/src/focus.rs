//! Focus ring and AccessKit focus update support.

use std::cell::Cell;

use accesskit::{NodeId as AccessNodeId, TreeId, TreeUpdate};
use sigil_hal::Point;
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::{SmallVec, smallvec};

use crate::EventHandlers;

mod scope;
mod targets;

mod request;

pub use request::request_focus;
pub(crate) use request::take_focus_requests;

/// Directional focus-navigation intent (dpad / analog-stick shaped).
///
/// Distinct from Tab order: Sigil stays device-agnostic and never reads
/// hardware directly. The host maps a gamepad dpad, arrow keys, or a
/// thumbstick's dominant axis to one of these four variants and calls
/// [`crate::EventRouter::focus_intent`]; Sigil resolves the target purely
/// from laid-out node rects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusIntent {
    /// Move focus to the nearest focusable node above the current one.
    Up,
    /// Move focus to the nearest focusable node below the current one.
    Down,
    /// Move focus to the nearest focusable node to the left.
    Left,
    /// Move focus to the nearest focusable node to the right.
    Right,
}

thread_local! {
    /// Whether the focus change currently being applied originated from the
    /// keyboard (Tab / programmatic) rather than a pointer press. The router
    /// stamps this immediately before each focus mutation; widget focus-in
    /// handlers read it to implement `:focus-visible` — show a focus ring on
    /// keyboard focus, but not on a mouse click.
    static FOCUS_FROM_KEYBOARD: Cell<bool> = const { Cell::new(true) };
}

/// Record the source of the focus change about to be applied. `true` =
/// keyboard / programmatic (ring-visible); `false` = pointer press.
pub fn note_focus_source(keyboard: bool) {
    FOCUS_FROM_KEYBOARD.with(|cell| cell.set(keyboard));
}

/// Whether the most recently stamped focus change came from the keyboard.
/// Read by widget focus-in handlers to gate the focus-visible ring.
#[must_use]
pub fn focus_from_keyboard() -> bool {
    FOCUS_FROM_KEYBOARD.with(Cell::get)
}

/// One entry on the focus-scope stack.
///
/// When a scope (e.g. a modal dialog) becomes active, the manager
/// pushes the scope's root node along with the focus that was active
/// at the moment of push. When the scope deactivates, the manager
/// pops the entry and attempts to restore focus to `restore_to`.
#[derive(Clone, Copy, Debug)]
struct FocusScopeEntry {
    /// Subtree root carrying the `a11y.focus_scope` marker.
    scope_root: NodeId,
    /// Node that was focused at push time. `None` if nothing was
    /// focused when the scope opened.
    restore_to: Option<NodeId>,
}

/// Focus state and lazily rebuilt tab ring.
#[derive(Debug)]
pub(crate) struct FocusManager {
    focused: Option<NodeId>,
    ring: Vec<NodeId>,
    ring_generation: u64,
    /// Layout generation the ring was built against — a reactive
    /// `Display::None` flip changes layout WITHOUT a kernel mutation, so
    /// the ring must rebuild on either counter moving.
    ring_layout_generation: u64,
    scope_stack: Vec<FocusScopeEntry>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focused: None,
            ring: Vec::new(),
            ring_generation: u64::MAX,
            ring_layout_generation: u64::MAX,
            scope_stack: Vec::new(),
        }
    }
}

impl FocusManager {
    pub(crate) const fn focus(&self) -> Option<NodeId> {
        self.focused
    }

    /// Depth of the active focus-scope stack. `0` means no scope is
    /// active and focus operates against the whole document.
    #[cfg(test)]
    pub(crate) const fn scope_depth(&self) -> usize {
        self.scope_stack.len()
    }

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(crate) fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
        let target = target.filter(|id| cx.get(*id).is_some());
        if self.focused == target {
            return;
        }
        let previous = self.focused;
        self.focused = target;
        if let Some(previous) = previous {
            fire_focus_out(cx, previous);
        }
        if let Some(next) = target {
            fire_focus_in(cx, next);
        }
    }

    pub(crate) fn focus_next(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        self.ensure_ring(cx, layout);
        let Some(next) = next_in_ring(&self.ring, self.focused) else {
            self.set_focus(cx, None);
            return;
        };
        self.set_focus(cx, Some(next));
    }

    pub(crate) fn focus_prev(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        self.ensure_ring(cx, layout);
        let Some(next) = prev_in_ring(&self.ring, self.focused) else {
            self.set_focus(cx, None);
            return;
        };
        self.set_focus(cx, Some(next));
    }

    /// Resolve `intent` spatially against laid-out node rects and move
    /// focus to the winning candidate. Falls back to Tab-order next/prev
    /// (Down/Right → next, Up/Left → prev) when nothing lies strictly in
    /// that direction — including when nothing is currently focused —
    /// so directional navigation never dead-ends.
    pub(crate) fn focus_intent(&mut self, cx: &mut Cx, layout: &LayoutEngine, intent: FocusIntent) {
        self.ensure_ring(cx, Some(layout));
        if let Some(target) = self.nearest_in_intent(cx, layout, intent) {
            self.set_focus(cx, Some(target));
            return;
        }
        match intent {
            FocusIntent::Down | FocusIntent::Right => self.focus_next(cx, Some(layout)),
            FocusIntent::Up | FocusIntent::Left => self.focus_prev(cx, Some(layout)),
        }
    }

    /// Rank every other node on the ring that lies strictly in `intent`'s
    /// direction from the focused node's rect center. Cost is
    /// travel-axis distance plus 2x the cross-axis deviation — the
    /// standard Manhattan-biased TV-navigation heuristic. Ties (equal
    /// cost) are broken by ring order: the first-encountered candidate
    /// wins since later matches must be strictly better to replace it.
    fn nearest_in_intent(
        &self,
        cx: &Cx,
        layout: &LayoutEngine,
        intent: FocusIntent,
    ) -> Option<NodeId> {
        let focused = self.focused?;
        let origin = node_center(cx, layout, focused)?;
        let mut best: Option<(f32, NodeId)> = None;
        for &candidate in &self.ring {
            if candidate == focused {
                continue;
            }
            let Some(center) = node_center(cx, layout, candidate) else {
                continue;
            };
            let Some(cost) = intent_cost(intent, origin, center) else {
                continue;
            };
            let better = best.is_none_or(|(best_cost, _)| cost < best_cost);
            if better {
                best = Some((cost, candidate));
            }
        }
        best.map(|(_, id)| id)
    }

    pub(crate) fn a11y_focus_update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            tree_id: TreeId::ROOT,
            focus: self.focused.map_or(AccessNodeId(0), NodeId::to_accesskit),
        }
    }

    fn ensure_ring(&mut self, cx: &Cx, layout: Option<&LayoutEngine>) {
        let layout_generation = layout.map_or(u64::MAX, LayoutEngine::generation);
        if self.ring_generation == cx.mutation_generation()
            && self.ring_layout_generation == layout_generation
        {
            return;
        }
        let mut ring = self.build_scoped_ring(cx);
        // Nodes without a current-frame layout result sit in
        // Display::None subtrees — invisible, un-hittable, and
        // therefore not tabbable either.
        if let Some(layout) = layout {
            ring.retain(|id| layout.result(*id).is_some());
        }
        self.ring = ring;
        self.ring_generation = cx.mutation_generation();
        self.ring_layout_generation = layout_generation;
    }

    /// Build the tab ring, confined to the topmost active scope when
    /// one exists. The walk starts at the scope root rather than at
    /// `cx.roots()`, so focusables outside the scope are excluded.
    fn build_scoped_ring(&self, cx: &Cx) -> Vec<NodeId> {
        let mut explicit = Vec::<FocusCandidate>::new();
        let mut implicit = Vec::<FocusCandidate>::new();
        let mut order = 0_usize;

        let walk_roots: SmallVec<[NodeId; 4]> = self.scope_stack.last().map_or_else(
            || cx.roots().iter().copied().collect(),
            |top| smallvec![top.scope_root],
        );
        for root in walk_roots {
            collect_focusable(cx, root, &mut order, &mut explicit, &mut implicit);
        }

        explicit.sort_by_key(|candidate| (candidate.tab_index, candidate.order));
        explicit
            .into_iter()
            .chain(implicit)
            .map(|candidate| candidate.id)
            .collect()
    }
}

fn fire_focus_in(cx: &Cx, id: NodeId) {
    if let Some(handlers) = cx
        .get(id)
        .and_then(|node| node.get_extension::<EventHandlers>())
    {
        handlers.invoke_focus_in();
    }
}

fn fire_focus_out(cx: &Cx, id: NodeId) {
    if let Some(handlers) = cx
        .get(id)
        .and_then(|node| node.get_extension::<EventHandlers>())
    {
        handlers.invoke_focus_out();
    }
}

/// Center point of `id`'s laid-out rect, in root coordinates. `None` if
/// the node is absent from the tree or has no computed rect.
fn node_center(cx: &Cx, layout: &LayoutEngine, id: NodeId) -> Option<Point> {
    cx.get(id)?;
    let rect = layout.result(id)?.rect();
    Some(Point::new(
        rect.origin.x + rect.width / 2.0,
        rect.origin.y + rect.height / 2.0,
    ))
}

/// Cost of moving from `origin` to `candidate` under `intent`, or `None`
/// if `candidate` does not lie strictly in `intent`'s direction. Cost is
/// travel-axis distance plus 2x the absolute cross-axis deviation — the
/// standard Manhattan-biased heuristic for dpad/TV focus navigation.
fn intent_cost(intent: FocusIntent, origin: Point, candidate: Point) -> Option<f32> {
    let (travel, cross) = match intent {
        FocusIntent::Right => (candidate.x - origin.x, candidate.y - origin.y),
        FocusIntent::Left => (origin.x - candidate.x, candidate.y - origin.y),
        FocusIntent::Down => (candidate.y - origin.y, candidate.x - origin.x),
        FocusIntent::Up => (origin.y - candidate.y, candidate.x - origin.x),
    };
    (travel > 0.0).then(|| 2.0_f32.mul_add(cross.abs(), travel))
}

fn collect_focusable(
    cx: &Cx,
    id: NodeId,
    order: &mut usize,
    explicit: &mut Vec<FocusCandidate>,
    implicit: &mut Vec<FocusCandidate>,
) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node.a11y.focusable {
        let candidate = FocusCandidate {
            id,
            tab_index: node.a11y.tab_index.unwrap_or(0),
            order: *order,
        };
        *order = order.saturating_add(1);
        match node.a11y.tab_index {
            Some(index) if index >= 0 => explicit.push(candidate),
            None => implicit.push(candidate),
            Some(_) => {}
        }
    }
    for child in node.children().iter().copied() {
        collect_focusable(cx, child, order, explicit, implicit);
    }
}

fn next_in_ring(ring: &[NodeId], focused: Option<NodeId>) -> Option<NodeId> {
    if ring.is_empty() {
        return None;
    }
    let index = focused
        .and_then(|focused| ring.iter().position(|id| *id == focused))
        .map_or(0, |index| (index + 1) % ring.len());
    ring.get(index).copied()
}

fn prev_in_ring(ring: &[NodeId], focused: Option<NodeId>) -> Option<NodeId> {
    if ring.is_empty() {
        return None;
    }
    let index = focused
        .and_then(|focused| ring.iter().position(|id| *id == focused))
        .map_or_else(
            || ring.len().saturating_sub(1),
            |index| previous_index(index, ring.len()),
        );
    ring.get(index).copied()
}

const fn previous_index(index: usize, len: usize) -> usize {
    if index == 0 { len - 1 } else { index - 1 }
}

#[derive(Clone, Copy)]
struct FocusCandidate {
    id: NodeId,
    tab_index: i32,
    order: usize,
}
