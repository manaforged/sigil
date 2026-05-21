//! Edit modal — title + description text fields with Save/Cancel.

use sigil::prelude::*;

use crate::state::BoardState;

pub(crate) fn edit_modal(state: BoardState) -> NodeBuilder {
    // The modal widget returns a NodeBuilder wrapping itself in
    // `into_node_builder`; we drive it through the impl IntoNode
    // pathway as a child of a passthrough container so the host tree
    // can attach it under the root without exposing the modal's
    // builder-only `.show()` requirement to the caller.
    container().layout(LayoutStyle::block()).child(
        modal()
            .show(state.modal_open)
            .title("Edit card")
            .escape_closes(true)
            .dismiss_on_backdrop(false)
            .trap(true)
            .child(modal_body(state)),
    )
}

fn modal_body(state: BoardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0))
        .child(label("Title"))
        .child(
            text_field(state.edit_title)
                .placeholder("Card title")
                .submit(move |_| state.commit_modal()),
        )
        .child(label("Description"))
        .child(
            text_field(state.edit_description)
                .placeholder("Description")
                .submit(move |_| state.commit_modal()),
        )
        .child(modal_actions(state))
}

fn modal_actions(state: BoardState) -> NodeBuilder {
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
