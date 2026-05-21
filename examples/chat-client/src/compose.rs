//! Compose box — multi-line [`text_edit`] over `state.compose_draft`,
//! with a "Send" button that drains the draft into the selected
//! channel's message log.
//!
//! `text_edit` (not `text_field`) is the right pick because chat
//! drafts can carry newline characters. The widget docs flag that the
//! visible line layout is a snapshot at build time; for this dogfood
//! demo a single growing draft is fine — when the user hits Send the
//! whole tree rebuilds anyway via the rebound channels signal.

use sigil::prelude::*;

use crate::state::ChatState;

const COMPOSE_HEIGHT: f32 = 96.0;

pub(crate) fn compose_box(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(6.0).padding(6.0))
        .class("w-full rounded-md bg-surface")
        .child(label("Compose"))
        .child(
            sigil::widgets::text_edit(state.compose_draft)
                .placeholder("Type a message — Enter for newline, Ctrl+Enter to send.")
                .submit(move |_| state.send_compose())
                .height(COMPOSE_HEIGHT),
        )
        .child(compose_actions(state))
}

fn compose_actions(state: ChatState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::End),
        )
        .child(
            button("Send")
                .primary()
                .activate(move || state.send_compose()),
        )
}
