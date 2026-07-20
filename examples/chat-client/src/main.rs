//! Tier-5 kitchen-sink chat-client app.
//!
//! Exercises all five tier-5 widgets live together: a left sidebar
//! [`virtual_list`] of channels, a horizontal [`splitter`] separating
//! sidebar from chat pane, a [`tabs`] strip routing the chat pane to
//! Messages / Members / Pinned, a [`text_edit`] compose box at the
//! bottom of the Messages tab, and a [`modal`] for "Channel settings"
//! containing two [`text_field`]s (channel name + topic).

#![allow(clippy::similar_names, clippy::assigning_clones)]

mod chat_pane;
mod compose;
mod settings_modal;
mod sidebar;
mod state;

#[cfg(test)]
mod tests;

use sigil::prelude::*;

use crate::chat_pane::chat_pane;
use crate::settings_modal::settings_modal;
use crate::sidebar::sidebar_pane;
use crate::state::ChatState;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - chat-client", 1100, 720), app_tree)
}

/// Build the full app tree under `cx`. Exposed `pub(crate)` so the
/// headless smoke tests can call it with a fresh `Cx` instead of
/// spinning up a window.
pub(crate) fn app_tree(cx: &mut Cx) -> NodeId {
    let state = ChatState::seed();
    cx.add(root_shell(state))
}

pub(crate) fn root_shell(state: ChatState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Chat client")
        .layout(LayoutStyle::flex_column().gap(8.0))
        .class(classes!("w-full h-full p-4 bg-surface"))
        .child(top_bar(state))
        .child(work_area(state))
        .child(status_bar(state))
        .child(settings_modal(state))
}

fn top_bar(state: ChatState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(48.0)
                .gap(12.0)
                .align_items(AlignItems::Center)
                .padding(8.0),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label("Chat"))
        .child(
            button("Channel settings")
                .primary()
                .activate(move || state.open_channel_settings()),
        )
}

fn work_area(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().grow(1.0))
        .class(classes!("w-full"))
        .child(
            sigil::widgets::splitter(sigil::widgets::SplitterOrientation::Horizontal)
                .child(sidebar_pane(state), 240.0)
                .min_size(0, 180.0)
                .max_size(0, 420.0)
                .child(chat_pane(state), 760.0)
                .min_size(1, 360.0)
                .sizes_bind(state.split_sizes),
        )
}

fn status_bar(state: ChatState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .padding(8.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label(state.status))
}
