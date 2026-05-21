//! Headless smoke tests — build the app tree without a window, then
//! mutate the modal-open signal and rebuild to assert the modal path
//! also constructs cleanly.

#![allow(clippy::expect_used)]

use sigil::testing::with_cx;

use crate::app_tree;
use crate::state::BoardState;

#[test]
fn app_tree_builds_without_panic() {
    with_cx("kanban-tree-builds", |cx| {
        let root = app_tree(cx);
        // `NodeId` is non-null by construction; assert the cx has at
        // least one child reachable from root so we know the tree is
        // populated, not a bare container.
        assert!(
            cx.first_child(root).is_some(),
            "app_tree must populate the root with children",
        );
    });
}

#[test]
fn modal_open_rebuilds_cleanly() {
    with_cx("kanban-modal-rebuild", |cx| {
        let state = BoardState::seed();

        // Mutate the modal-open signal as the user would when they
        // click "New card", then build the full tree against the
        // mutated state. The build path is what we're asserting on.
        state.open_new_card();
        assert!(state.modal_open.peek(), "open_new_card must set modal_open");

        let root = cx.add(crate::root_shell(state));
        assert!(
            cx.first_child(root).is_some(),
            "modal-open tree must still construct children",
        );

        // Cancel restores the closed state without panicking.
        state.cancel_modal();
        assert!(
            !state.modal_open.peek(),
            "cancel_modal must clear modal_open"
        );
    });
}
