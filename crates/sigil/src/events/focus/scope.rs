use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use crate::style::StyleBinding;
use smallvec::SmallVec;

use crate::events::hit_test::visible;

use super::{FocusManager, FocusScopeEntry};

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
            self.focused = top
                .restore_to
                .filter(|id| visible(cx, *id))
                .or_else(|| first_focusable_in_document_order(cx));
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
            self.scope_stack.push(FocusScopeEntry {
                scope_root,
                restore_to: self.focused,
            });
            if cx
                .get(scope_root)
                .is_some_and(|node| node.a11y.is_focusable())
            {
                self.set_focus(cx, Some(scope_root));
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

fn first_focusable_in_document_order(cx: &Cx) -> Option<NodeId> {
    for root in cx.roots().iter().copied() {
        if let Some(id) = first_focusable_in_subtree(cx, root) {
            return Some(id);
        }
    }
    None
}

fn first_focusable_in_subtree(cx: &Cx, id: NodeId) -> Option<NodeId> {
    if !visible(cx, id) {
        return None;
    }
    let node = cx.get(id)?;
    if node.a11y.is_focusable() {
        return Some(id);
    }
    for child in node.children().iter().copied() {
        if let Some(found) = first_focusable_in_subtree(cx, child) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;
