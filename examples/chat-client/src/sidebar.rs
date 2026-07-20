//! Sidebar pane — header label + channel [`virtual_list`].

use sigil::prelude::*;

use crate::state::ChatState;

const ROW_HEIGHT: f32 = 36.0;
const VIEWPORT_HEIGHT: f32 = 520.0;

pub(crate) fn sidebar_pane(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).padding(8.0))
        .class(classes!("h-full rounded-md bg-surface-hover"))
        .child(label("Channels"))
        .child(channel_list(state))
}

fn channel_list(state: ChatState) -> NodeBuilder {
    // Snapshot the channel-name list at build time. Adding channels
    // would require a rebuild of the surrounding tree; for this
    // dogfood pass that's fine — channel set is stable per session.
    let rows: Vec<String> = state
        .channels
        .peek()
        .iter()
        .map(|c| c.name.clone())
        .collect();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0))
        .child(
            sigil::widgets::virtual_list(rows, move |idx, name| channel_row(state, idx, name))
                .row_height(ROW_HEIGHT)
                .viewport_height(VIEWPORT_HEIGHT)
                .selection(sigil::widgets::SelectionMode::Single),
        )
}

fn channel_row(state: ChatState, idx: usize, name: &str) -> NodeBuilder {
    let label_text = name.to_owned();
    container()
        .role(Role::Button)
        .label(name.to_owned())
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(ROW_HEIGHT)
                .padding(6.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("w-full rounded-sm bg-surface"))
        .pointer_down(move |ev| {
            state.select_channel(idx);
            ev.stop_propagation();
        })
        .child(label(label_text))
}
