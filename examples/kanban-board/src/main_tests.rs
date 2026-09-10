#![allow(clippy::expect_used)]

use sigil::testing::with_cx;

use crate::app_tree;
use crate::state::BoardState;

#[test]
fn app_tree_builds_without_panic() {
    with_cx("kanban-tree-builds", |cx| {
        let root = app_tree(cx);
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

        state.open_new_card();
        assert!(state.modal_open.peek(), "open_new_card must set modal_open");

        let root = cx.add(crate::root_shell(state));
        assert!(
            cx.first_child(root).is_some(),
            "modal-open tree must still construct children",
        );

        state.cancel_modal();
        assert!(
            !state.modal_open.peek(),
            "cancel_modal must clear modal_open"
        );
    });
}

#[test]
fn move_card_changes_column_counts() {
    with_cx("kanban-move-card", |_cx| {
        let state = BoardState::seed();
        let before = state.columns.peek();
        let todo = before[0].len();
        let doing = before[1].len();
        state.move_card(0, 0, 1);
        let after = state.columns.peek();
        assert_eq!(after[0].len(), todo - 1);
        assert_eq!(after[1].len(), doing + 1);
    });
}
