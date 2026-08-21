//! Focus-scope stack reconciliation (modal / popup push-pop-restore).
//!
//! Split out of `focus.rs` — that file grew back over the 400-line
//! hygiene cap after the layout-aware scope/ring work; this is the
//! scope-stack-management concern (distinct from tab-ring construction,
//! which stays in the parent module) split into its own child module,
//! matching the `focus/targets.rs` precedent already in this crate.

use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::SmallVec;

use super::{FocusManager, FocusScopeEntry};

impl FocusManager {
    /// Reconcile the scope stack against the current tree state.
    ///
    /// Walks every root, collects nodes with `a11y.focus_scope` set in
    /// document order, then:
    ///
    /// - Pops trailing entries whose `scope_root` is no longer present
    ///   in the collected list (or is no longer in the tree at all).
    /// - For each pop, attempts to restore focus to `restore_to` if
    ///   that node still exists; otherwise leaves focus untouched
    ///   (Task 5 adds the stale-node fallback).
    /// - Pushes new scope nodes that appear after the current stack tip.
    ///
    /// `sync_scopes` is called at the start of every `EventRouter::route`
    /// so scope membership tracks tree state without explicit widget
    /// lifecycle hooks.
    #[allow(clippy::manual_contains, clippy::needless_pass_by_ref_mut)]
    pub(crate) fn sync_scopes(&mut self, cx: &mut Cx, layout: Option<&LayoutEngine>) {
        let mut current = Vec::new();
        for root in cx.roots().iter().copied() {
            collect_scope_roots(cx, root, &mut current);
        }
        // A scope root with no layout result sits in a Display::None
        // subtree: invisible and un-hittable, so it must not hold the
        // focus stack either — a dismissed modal would otherwise trap
        // Tab inside an invisible dialog. `None` (unit tests) skips
        // the visibility filter.
        if let Some(layout) = layout {
            current.retain(|id| layout.result(*id).is_some());
        }

        // Pop entries no longer in the collected set. We pop from the
        // top; an entry survives iff its scope_root is still present
        // AND the stack-prefix below it matches the collected order.
        while let Some(top) = self.scope_stack.last().copied() {
            let still_present = current.iter().any(|id| *id == top.scope_root);
            if still_present {
                break;
            }
            self.scope_stack.pop();
            // Restoration with stale-node fallback.
            self.focused = top
                .restore_to
                .filter(|id| cx.get(*id).is_some())
                .or_else(|| first_focusable_in_document_order(cx));
        }

        // Push any newly-appeared scopes. New = present in `current`
        // beyond what the stack already covers, in document order.
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
        }
    }
}

/// Walks the subtree rooted at `id` and appends every node whose
/// `a11y.focus_scope` field is `Some(_)`, in document order.
///
/// Scopes can nest - both the outer and inner scope nodes are
/// reported; the consumer decides how to handle nesting.
fn collect_scope_roots(cx: &Cx, id: NodeId, out: &mut Vec<NodeId>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node.a11y.focus_scope.is_some() {
        out.push(id);
    }
    for child in node.children().iter().copied() {
        collect_scope_roots(cx, child, out);
    }
}

/// Find the first focusable node in document order across all roots.
/// Used as a fallback when a focus-scope pops and the prior focus
/// target is no longer in the tree.
fn first_focusable_in_document_order(cx: &Cx) -> Option<NodeId> {
    for root in cx.roots().iter().copied() {
        if let Some(id) = first_focusable_in_subtree(cx, root) {
            return Some(id);
        }
    }
    None
}

fn first_focusable_in_subtree(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.focusable {
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
