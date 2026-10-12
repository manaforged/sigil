use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::geometry::Point;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use accesskit::{NodeId as AccessNodeId, TreeId, TreeUpdate};
use smallvec::{SmallVec, smallvec};

use crate::events::handlers::{EventHandlers, dispatch};
use crate::events::hit_test::visible;

mod scope;
mod targets;

mod request;
mod reveal;

pub(crate) use request::{Autofocus, focus_on_trigger, take_focus_requests, take_reveal_requests};
pub use request::{request_focus, request_reveal};
pub(crate) use reveal::{ScrollReveal, reveal_focus};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FocusIntent {
    Up,
    Down,
    Left,
    Right,
}

thread_local! {
    static FOCUS_FROM_KEYBOARD: RefCell<Option<Rc<Cell<bool>>>> = const { RefCell::new(None) };
    static FALLBACK_FOCUS_SOURCE: Cell<bool> = const { Cell::new(true) };
}

pub(crate) fn note_focus_source(keyboard: bool) {
    let target = FOCUS_FROM_KEYBOARD.with(|slot| slot.borrow().clone());
    if let Some(cell) = target {
        cell.set(keyboard);
    } else {
        FALLBACK_FOCUS_SOURCE.set(keyboard);
    }
}

#[must_use]
pub fn focus_from_keyboard() -> bool {
    FOCUS_FROM_KEYBOARD
        .with(|slot| slot.borrow().clone())
        .map_or_else(|| FALLBACK_FOCUS_SOURCE.get(), |cell| cell.get())
}

pub(crate) fn swap_focus_source(target: Option<Rc<Cell<bool>>>) -> Option<Rc<Cell<bool>>> {
    FOCUS_FROM_KEYBOARD.with(|slot| core::mem::replace(&mut *slot.borrow_mut(), target))
}

#[derive(Clone, Copy, Debug)]
struct FocusScopeEntry {
    scope_root: NodeId,
    restore_to: Option<NodeId>,
}

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TabEdges {
    pub(crate) first: bool,
    pub(crate) last: bool,
}

#[derive(Debug)]
pub(crate) struct FocusManager {
    focused: Option<NodeId>,
    ring: Vec<NodeId>,
    ring_generation: u64,
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

    #[cfg(test)]
    pub(crate) const fn scope_depth(&self) -> usize {
        self.scope_stack.len()
    }

    #[allow(
        clippy::needless_pass_by_ref_mut,
        reason = "intentional, reviewed trade-off"
    )]
    pub(crate) fn set_focus(&mut self, cx: &mut Cx, target: Option<NodeId>) {
        let target = target.filter(|id| cx.get(*id).is_some());
        if self.focused == target {
            return;
        }
        let previous = self.focused;
        self.focused = target;
        if let Some(previous) = previous {
            let _ = dispatch(cx, previous, EventHandlers::invoke_focus_out);
        }
        if let Some(next) = target {
            let _ = dispatch(cx, next, EventHandlers::invoke_focus_in);
        }
    }

    pub(crate) fn focus_next(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        self.focus_step(cx, layout, true);
    }

    pub(crate) fn focus_prev(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        self.focus_step(cx, layout, false);
    }

    fn focus_step(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>, forward: bool) {
        self.ensure_ring(cx, layout);
        let next = step_in_ring(&self.enabled_ring(cx), self.focused, forward);
        self.set_focus(cx, next);
    }

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

    fn nearest_in_intent(
        &self,
        cx: &Cx,
        layout: &LayoutEngine,
        intent: FocusIntent,
    ) -> Option<NodeId> {
        let focused = self.focused?;
        let origin = node_center(cx, layout, focused)?;
        self.ring
            .iter()
            .copied()
            .filter(|&candidate| {
                candidate != focused
                    && visible(cx, candidate)
                    && cx
                        .get(candidate)
                        .is_some_and(|node| node.a11y.is_focusable())
            })
            .filter_map(|candidate| {
                let center = node_center(cx, layout, candidate)?;
                Some((intent_cost(intent, origin, center)?, candidate))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    pub(crate) fn a11y_focus_update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            tree_id: TreeId::ROOT,
            focus: self.focused.map_or(AccessNodeId(0), NodeId::to_accesskit),
        }
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn tab_edges(&self, cx: &Cx) -> TabEdges {
        let ring = self.enabled_ring(cx);
        let Some(focused) = self.focused else {
            return TabEdges {
                first: ring.is_empty(),
                last: ring.is_empty(),
            };
        };
        TabEdges {
            first: ring.first().is_none_or(|id| *id == focused),
            last: ring.last().is_none_or(|id| *id == focused),
        }
    }

    fn enabled_ring(&self, cx: &Cx) -> Vec<NodeId> {
        self.ring
            .iter()
            .copied()
            .filter(|id| {
                visible(cx, *id)
                    && cx.get(*id).is_some_and(|node| {
                        node.a11y.is_focusable()
                            && (self.focused == Some(*id)
                                || node.a11y.tab_stop.is_none_or(|stop| stop.peek()))
                    })
            })
            .collect()
    }

    fn ensure_ring(&mut self, cx: &Cx, layout: Option<&LayoutEngine>) {
        let layout_generation = layout.map_or(u64::MAX, LayoutEngine::generation);
        if self.ring_generation == cx.mutation_generation()
            && self.ring_layout_generation == layout_generation
        {
            return;
        }
        let mut ring = self.build_scoped_ring(cx);
        if let Some(layout) = layout {
            ring.retain(|id| layout.result(*id).is_some());
        }
        self.ring = ring;
        self.ring_generation = cx.mutation_generation();
        self.ring_layout_generation = layout_generation;
    }

    fn build_scoped_ring(&self, cx: &Cx) -> Vec<NodeId> {
        let mut explicit = Vec::new();
        let mut implicit = Vec::new();
        let walk_roots: SmallVec<[NodeId; 4]> = self.scope_stack.last().map_or_else(
            || cx.roots().iter().copied().collect(),
            |top| smallvec![top.scope_root],
        );
        for root in walk_roots {
            collect_focusable(cx, root, &mut explicit, &mut implicit);
        }
        explicit.sort_by_key(|(tab_index, _)| *tab_index);
        explicit
            .into_iter()
            .map(|(_, id)| id)
            .chain(implicit)
            .collect()
    }
}

fn node_center(cx: &Cx, layout: &LayoutEngine, id: NodeId) -> Option<Point> {
    cx.get(id)?;
    let rect = layout.result(id)?.rect();
    Some(Point::new(
        rect.origin.x + rect.width / 2.0,
        rect.origin.y + rect.height / 2.0,
    ))
}

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
    explicit: &mut Vec<(i32, NodeId)>,
    implicit: &mut Vec<NodeId>,
) {
    if !visible(cx, id) {
        return;
    }
    let Some(node) = cx.get(id) else {
        return;
    };
    if node.a11y.focusable {
        match node.a11y.tab_index {
            Some(index) if index >= 0 => explicit.push((index, id)),
            None => implicit.push(id),
            Some(_) => {}
        }
    }
    for child in node.children().iter().copied() {
        collect_focusable(cx, child, explicit, implicit);
    }
}

fn step_in_ring(ring: &[NodeId], focused: Option<NodeId>, forward: bool) -> Option<NodeId> {
    let len = ring.len();
    let index = match focused.and_then(|focused| ring.iter().position(|id| *id == focused)) {
        Some(index) if forward => (index + 1) % len,
        Some(index) => (index + len - 1) % len,
        None if forward => 0,
        None => len.checked_sub(1)?,
    };
    ring.get(index).copied()
}

#[cfg(test)]
#[path = "focus/edges_tests.rs"]
mod edges_tests;
