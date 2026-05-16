//! Focus ring and AccessKit focus update support.

use accesskit::{NodeId as AccessNodeId, TreeId, TreeUpdate};
use sigil_kernel::{Cx, NodeId};

use crate::EventHandlers;

/// Focus state and lazily rebuilt tab ring.
#[derive(Debug)]
pub(crate) struct FocusManager {
    focused: Option<NodeId>,
    ring: Vec<NodeId>,
    ring_generation: u64,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focused: None,
            ring: Vec::new(),
            ring_generation: u64::MAX,
        }
    }
}

impl FocusManager {
    pub(crate) const fn focus(&self) -> Option<NodeId> {
        self.focused
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

    pub(crate) fn focus_next(&mut self, cx: &mut Cx) {
        self.ensure_ring(cx);
        let Some(next) = next_in_ring(&self.ring, self.focused) else {
            self.set_focus(cx, None);
            return;
        };
        self.set_focus(cx, Some(next));
    }

    pub(crate) fn focus_prev(&mut self, cx: &mut Cx) {
        self.ensure_ring(cx);
        let Some(next) = prev_in_ring(&self.ring, self.focused) else {
            self.set_focus(cx, None);
            return;
        };
        self.set_focus(cx, Some(next));
    }

    pub(crate) fn a11y_focus_update(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            tree_id: TreeId::ROOT,
            focus: self.focused.map_or(AccessNodeId(0), NodeId::to_accesskit),
        }
    }

    fn ensure_ring(&mut self, cx: &Cx) {
        if self.ring_generation == cx.mutation_generation() {
            return;
        }
        self.ring = build_ring(cx);
        self.ring_generation = cx.mutation_generation();
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

fn build_ring(cx: &Cx) -> Vec<NodeId> {
    let mut explicit = Vec::<FocusCandidate>::new();
    let mut implicit = Vec::<FocusCandidate>::new();
    let mut order = 0_usize;
    for root in cx.roots().iter().copied() {
        collect_focusable(cx, root, &mut order, &mut explicit, &mut implicit);
    }
    explicit.sort_by_key(|candidate| (candidate.tab_index, candidate.order));
    explicit
        .into_iter()
        .chain(implicit)
        .map(|candidate| candidate.id)
        .collect()
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
    for child in node.children.iter().copied() {
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
