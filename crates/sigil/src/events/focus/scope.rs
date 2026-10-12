use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use crate::style::StyleBinding;
use smallvec::SmallVec;

use crate::events::focus::targets::is_self_or_ancestor;
use crate::events::hit_test::visible;

use super::{Autofocus, FocusManager, FocusScopeEntry};

impl FocusManager {
    #[allow(
        clippy::manual_contains,
        clippy::needless_pass_by_ref_mut,
        reason = "intentional, reviewed trade-off"
    )]
    pub(crate) fn sync_scopes(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        let mut current = Vec::new();
        for root in cx.roots().iter().copied() {
            collect_scope_roots(cx, root, &mut current);
        }
        if let Some(layout) = layout {
            current.retain(|id| layout.result(*id).is_some());
        }

        while let Some(top) = self.scope_stack.last().copied() {
            let still_present = current.iter().any(|id| *id == top.scope_root);
            if still_present {
                break;
            }
            self.scope_stack.pop();
            self.ring_generation = u64::MAX;
            self.focused = top.restore_to.filter(|id| visible(cx, *id)).or_else(|| {
                first_in_document_order(cx, |id| {
                    cx.get(id).is_some_and(|node| node.a11y.is_focusable())
                })
            });
        }

        let stack_roots: SmallVec<[NodeId; 4]> = self
            .scope_stack
            .iter()
            .map(|entry| entry.scope_root)
            .collect();
        for scope_root in current {
            if stack_roots.iter().any(|id| *id == scope_root) {
                continue;
            }
            let focused_inside = self
                .focused
                .is_some_and(|id| is_self_or_ancestor(cx, scope_root, id));
            self.scope_stack.push(FocusScopeEntry {
                scope_root,
                restore_to: self.focused,
            });
            if !focused_inside && let Some(target) = initial_focus(cx, scope_root) {
                self.set_focus(cx, Some(target));
            }
            self.ring_generation = u64::MAX;
        }
    }
}

fn collect_scope_roots(cx: &Cx, id: NodeId, out: &mut Vec<NodeId>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node
        .get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
    {
        return;
    }
    if node.a11y.focus_scope.is_some() {
        out.push(id);
    }
    for child in node.children().iter().copied() {
        collect_scope_roots(cx, child, out);
    }
}

fn initial_focus(cx: &Cx, scope_root: NodeId) -> Option<NodeId> {
    first_in_subtree(cx, scope_root, &|id| {
        cx.get(id).is_some_and(|node| {
            node.a11y.is_focusable() && node.get_extension::<Autofocus>().is_some()
        })
    })
    .or_else(|| {
        cx.get(scope_root)
            .is_some_and(|node| node.a11y.is_focusable())
            .then_some(scope_root)
    })
}

pub(super) fn first_in_document_order(cx: &Cx, accept: impl Fn(NodeId) -> bool) -> Option<NodeId> {
    cx.roots()
        .iter()
        .copied()
        .find_map(|root| first_in_subtree(cx, root, &accept))
}

fn first_in_subtree(cx: &Cx, id: NodeId, accept: &impl Fn(NodeId) -> bool) -> Option<NodeId> {
    if !visible(cx, id) {
        return None;
    }
    if accept(id) {
        return Some(id);
    }
    cx.get(id)?
        .children()
        .iter()
        .copied()
        .find_map(|child| first_in_subtree(cx, child, accept))
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;
