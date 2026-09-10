use sigil::prelude::*;

use crate::state::ChatState;

const COMPOSE_HEIGHT: f32 = 96.0;

pub(crate) fn compose_box(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(6.0).padding(6.0))
        .class(classes!("w-full rounded-md bg-surface"))
        .child(label("Compose"))
        .child(
            text_edit(state.compose_draft)
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
