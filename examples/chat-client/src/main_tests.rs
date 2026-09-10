#![allow(clippy::expect_used)]

use sigil::testing::with_cx;

use crate::app_tree;
use crate::state::ChatState;

#[test]
fn app_tree_builds_without_panic() {
    with_cx("chat-client-tree-builds", |cx| {
        let root = app_tree(cx);
        assert!(
            cx.first_child(root).is_some(),
            "app_tree must populate the root with children",
        );
    });
}

#[test]
fn modal_open_rebuilds_cleanly() {
    with_cx("chat-client-modal-rebuild", |cx| {
        let state = ChatState::seed();

        state.open_channel_settings();
        assert!(
            state.modal_open.peek(),
            "open_channel_settings must set modal_open",
        );

        let root = cx.add(crate::root_shell(state));
        assert!(
            cx.first_child(root).is_some(),
            "modal-open tree must still construct children",
        );

        state.cancel_modal();
        assert!(
            !state.modal_open.peek(),
            "cancel_modal must clear modal_open",
        );
    });
}

#[test]
fn send_compose_drains_draft_and_appends_message() {
    with_cx("chat-client-send-compose", |_cx| {
        let state = ChatState::seed();
        state.compose_draft.set(String::from("hello dogfood"));
        let before = state
            .channels
            .peek()
            .first()
            .map_or(0, |c| c.messages.len());
        state.send_compose();
        let after = state
            .channels
            .peek()
            .first()
            .map_or(0, |c| c.messages.len());
        assert_eq!(after, before + 1, "send must append exactly one message");
        assert!(
            state.compose_draft.peek().is_empty(),
            "send must drain the draft",
        );
    });
}
