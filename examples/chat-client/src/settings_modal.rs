use sigil::prelude::*;

use crate::state::ChatState;

pub(crate) fn settings_modal(state: ChatState) -> NodeBuilder {
    container().layout(LayoutStyle::block()).child(
        modal(state.modal_open)
            .title("Channel settings")
            .escape_closes(true)
            .dismiss_on_backdrop(false)
            .trap(true)
            .child(modal_body(state)),
    )
}

fn modal_body(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0))
        .child(label("Channel name"))
        .child(
            text_field(state.modal_name_draft)
                .placeholder("#channel-name")
                .submit(move |_| state.commit_modal()),
        )
        .child(label("Topic (first pinned line)"))
        .child(
            text_field(state.modal_topic_draft)
                .placeholder("What's this channel about?")
                .submit(move |_| state.commit_modal()),
        )
        .child(actions(state))
}

fn actions(state: ChatState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::End),
        )
        .child(button("Cancel").activate(move || state.cancel_modal()))
        .child(
            button("Save")
                .primary()
                .activate(move || state.commit_modal()),
        )
}
