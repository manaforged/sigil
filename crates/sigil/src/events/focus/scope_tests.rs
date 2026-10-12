use super::*;
use crate::kernel::{FocusScopeKind, NodeBuilder, container};
use crate::layout::*;

fn sized(side: f32) -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(side, side))
}

fn focusable() -> NodeBuilder {
    sized(10.0).focusable(true)
}

fn scope(kind: FocusScopeKind) -> NodeBuilder {
    sized(50.0).mutate(move |node| node.a11y.focus_scope = Some(kind))
}

#[test]
fn collect_scope_roots_finds_marked_nodes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        sized(100.0)
            .child(scope(FocusScopeKind::Modal))
            .child(sized(10.0)),
    );
    let modal = cx.nth_child(root, 0).expect("modal child should exist");

    let mut found = Vec::new();
    collect_scope_roots(&cx, root, &mut found);

    assert_eq!(found, vec![modal]);
}

#[test]
fn collect_scope_roots_handles_nested_scopes() {
    let mut cx = Cx::new_root();
    let root = cx
        .add(sized(100.0).child(scope(FocusScopeKind::Modal).child(scope(FocusScopeKind::Popup))));
    let outer = cx.nth_child(root, 0).expect("outer scope should exist");
    let inner = cx.nth_child(outer, 0).expect("inner scope should exist");

    let mut found = Vec::new();
    collect_scope_roots(&cx, root, &mut found);

    assert_eq!(found, vec![outer, inner]);
}

#[test]
fn sync_scopes_pushes_new_scope_with_restore_target() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        sized(100.0)
            .child(focusable())
            .child(scope(FocusScopeKind::Modal)),
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
    let root = cx.add(sized(100.0).child(focusable()));
    let outside = cx.nth_child(root, 0).expect("outside child should exist");

    let modal = cx.add_child(root, scope(FocusScopeKind::Modal));

    let mut focus = FocusManager::default();
    focus.set_focus(&mut cx, Some(outside));
    focus.sync_scopes(&mut cx, None);
    assert_eq!(focus.scope_depth(), 1);

    focus.set_focus(&mut cx, Some(modal));

    cx.remove(modal);
    focus.sync_scopes(&mut cx, None);

    assert_eq!(focus.scope_depth(), 0);
    assert_eq!(focus.focus(), Some(outside));
}

#[test]
fn tab_ring_is_confined_to_topmost_scope() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        sized(100.0).child(focusable()).child(focusable()).child(
            scope(FocusScopeKind::Modal)
                .child(focusable())
                .child(focusable()),
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
    assert_eq!(focus.focus(), Some(inside_a));
}

#[test]
fn stale_restore_target_falls_back_to_document_order() {
    let mut cx = Cx::new_root();
    let root = cx.add(sized(100.0).child(focusable()).child(focusable()));
    let a = cx.nth_child(root, 0).expect("first child should exist");
    let b = cx.nth_child(root, 1).expect("second child should exist");

    let mut focus = FocusManager::default();
    focus.set_focus(&mut cx, Some(a));

    let modal = cx.add_child(root, scope(FocusScopeKind::Modal));
    focus.sync_scopes(&mut cx, None);
    assert_eq!(focus.scope_depth(), 1);

    cx.remove(a);
    cx.remove(modal);
    focus.sync_scopes(&mut cx, None);

    assert_eq!(focus.scope_depth(), 0);
    assert_eq!(focus.focus(), Some(b));
}
