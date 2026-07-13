//! Focus-scope stack reconciliation (modal / popup push-pop-restore).
//!
//! Split out of `focus.rs` — that file grew back over the 400-line
//! hygiene cap after the layout-aware scope/ring work; this is the
//! scope-stack-management concern (distinct from tab-ring construction,
//! which stays in the parent module) split into its own child module,
//! matching the `focus/targets.rs` precedent already in this crate.

use kernel::{Cx, NodeId};
use layout::LayoutEngine;
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
mod tests {
    use super::*;
    use kernel::{FocusScopeKind, container};
    use layout::prelude::*;

    fn scope_marker(kind: FocusScopeKind) -> impl FnOnce(&mut kernel::Node) {
        move |node| {
            node.a11y.focus_scope = Some(kind);
        }
    }

    #[test]
    fn collect_scope_roots_finds_marked_nodes() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .child(
                    container()
                        .mutate(scope_marker(FocusScopeKind::Modal))
                        .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
                )
                .child(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0))),
        );
        let modal = cx.nth_child(root, 0).expect("modal child should exist");

        let mut found = Vec::new();
        collect_scope_roots(&cx, root, &mut found);

        assert_eq!(found, vec![modal]);
    }

    #[test]
    fn collect_scope_roots_handles_nested_scopes() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .child(
                    container()
                        .mutate(scope_marker(FocusScopeKind::Modal))
                        .child(
                            container()
                                .mutate(scope_marker(FocusScopeKind::Popup))
                                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                        )
                        .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
                ),
        );
        let outer = cx.nth_child(root, 0).expect("outer scope should exist");
        let inner = cx.nth_child(outer, 0).expect("inner scope should exist");

        let mut found = Vec::new();
        collect_scope_roots(&cx, root, &mut found);

        // Document order: outer first, then inner.
        assert_eq!(found, vec![outer, inner]);
    }

    #[test]
    fn sync_scopes_pushes_new_scope_with_restore_target() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                )
                .child(
                    container()
                        .mutate(scope_marker(FocusScopeKind::Modal))
                        .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
                ),
        );
        let outside = cx.nth_child(root, 0).expect("outside child should exist");
        let modal = cx.nth_child(root, 1).expect("modal child should exist");

        let mut focus = FocusManager::default();
        focus.set_focus(&mut cx, Some(outside));
        focus.sync_scopes(&mut cx, None);

        assert_eq!(focus.scope_depth(), 1);
        let entry = focus
            .scope_stack
            .last()
            .copied()
            .expect("scope entry should exist");
        assert_eq!(entry.scope_root, modal);
        assert_eq!(entry.restore_to, Some(outside));
    }

    #[test]
    fn sync_scopes_pops_scope_when_node_removed_and_restores_focus() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                ),
        );
        let outside = cx.nth_child(root, 0).expect("outside child should exist");

        // Add a modal subtree dynamically.
        let modal = cx.add_child(
            root,
            container()
                .mutate(scope_marker(FocusScopeKind::Modal))
                .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
        );

        let mut focus = FocusManager::default();
        focus.set_focus(&mut cx, Some(outside));
        focus.sync_scopes(&mut cx, None);
        assert_eq!(focus.scope_depth(), 1);

        // Set focus inside the modal subtree (simulate modal focus).
        focus.set_focus(&mut cx, Some(modal));

        // Remove the modal subtree.
        cx.remove(modal);
        focus.sync_scopes(&mut cx, None);

        assert_eq!(focus.scope_depth(), 0);
        assert_eq!(focus.focus(), Some(outside));
    }

    #[test]
    fn tab_ring_is_confined_to_topmost_scope() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                // Two focusables outside any scope.
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                )
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                )
                // Modal with two focusables inside.
                .child(
                    container()
                        .mutate(scope_marker(FocusScopeKind::Modal))
                        .layout(LayoutStyle::block().fixed_size(50.0, 50.0))
                        .child(
                            container()
                                .focusable(true)
                                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                        )
                        .child(
                            container()
                                .focusable(true)
                                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                        ),
                ),
        );
        let modal = cx.nth_child(root, 2).expect("modal child should exist");
        let inside_a = cx.nth_child(modal, 0).expect("inside_a should exist");
        let inside_b = cx.nth_child(modal, 1).expect("inside_b should exist");

        let mut focus = FocusManager::default();
        focus.sync_scopes(&mut cx, None);
        assert_eq!(focus.scope_depth(), 1);

        focus.focus_next(&mut cx, None);
        assert_eq!(focus.focus(), Some(inside_a));
        focus.focus_next(&mut cx, None);
        assert_eq!(focus.focus(), Some(inside_b));
        focus.focus_next(&mut cx, None);
        // Wrap inside the scope, not escape to outsiders.
        assert_eq!(focus.focus(), Some(inside_a));
    }

    #[test]
    fn stale_restore_target_falls_back_to_document_order() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                )
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                ),
        );
        let a = cx.nth_child(root, 0).expect("first child should exist");
        let b = cx.nth_child(root, 1).expect("second child should exist");

        let mut focus = FocusManager::default();
        focus.set_focus(&mut cx, Some(a));

        // Add a modal, sync pushes scope with restore_to = a.
        let modal = cx.add_child(
            root,
            container()
                .mutate(scope_marker(FocusScopeKind::Modal))
                .layout(LayoutStyle::block().fixed_size(50.0, 50.0)),
        );
        focus.sync_scopes(&mut cx, None);
        assert_eq!(focus.scope_depth(), 1);

        // Remove both the modal and the original focus target.
        cx.remove(a);
        cx.remove(modal);
        focus.sync_scopes(&mut cx, None);

        assert_eq!(focus.scope_depth(), 0);
        // Fallback: focus the first remaining document-order focusable.
        assert_eq!(focus.focus(), Some(b));
    }
}
