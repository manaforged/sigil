//! Chat pane — horizontal [`tabs`] across Messages / Members / Pinned,
//! routing to a per-tab content node. Messages embeds a
//! [`virtual_list`] of messages + the compose [`text_edit`].

use sigil::prelude::*;

use crate::compose::compose_box;
use crate::state::{ChatState, Message, TAB_MEMBERS, TAB_MESSAGES, TAB_PINNED, tab_title};

const MSG_ROW_HEIGHT: f32 = 56.0;
const MEMBER_ROW_HEIGHT: f32 = 28.0;
const PINNED_ROW_HEIGHT: f32 = 32.0;
const LIST_VIEWPORT_HEIGHT: f32 = 360.0;

pub(crate) fn chat_pane(state: ChatState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0).padding(8.0))
        .class("h-full rounded-md bg-surface-hover")
        .child(tab_strip(state))
}

fn tab_strip(state: ChatState) -> NodeBuilder {
    sigil::widgets::tabs(
        state.active_tab,
        vec![
            String::from(tab_title(TAB_MESSAGES)),
            String::from(tab_title(TAB_MEMBERS)),
            String::from(tab_title(TAB_PINNED)),
        ],
    )
    .label("Chat tabs")
    .content_for(TAB_MESSAGES, messages_tab(state))
    .content_for(TAB_MEMBERS, members_tab(state))
    .content_for(TAB_PINNED, pinned_tab(state))
    .into_node_builder_passthrough()
}

// `TabsBuilder` is `IntoNode` so it can be used as a child directly,
// but the surrounding container needs a `NodeBuilder`. Wrap it in a
// thin passthrough to keep the type uniform across the file.
trait TabsPassthrough {
    fn into_node_builder_passthrough(self) -> NodeBuilder;
}

impl TabsPassthrough for sigil::widgets::TabsBuilder {
    fn into_node_builder_passthrough(self) -> NodeBuilder {
        container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .child(self)
    }
}

fn messages_tab(state: ChatState) -> NodeBuilder {
    let idx = state.selected_channel.peek();
    let messages: Vec<Message> = state
        .channels
        .peek()
        .get(idx)
        .map(|c| c.messages.clone())
        .unwrap_or_default();
    let count = messages.len();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0))
        .child(label(format!("Messages ({count})")))
        .child(
            sigil::widgets::virtual_list(messages, message_row)
                .row_height(MSG_ROW_HEIGHT)
                .viewport_height(LIST_VIEWPORT_HEIGHT),
        )
        .child(compose_box(state))
}

fn message_row(_idx: usize, msg: &Message) -> NodeBuilder {
    let author = msg.author.clone();
    let body = msg.body.clone();
    container()
        .role(Role::ListItem)
        .label(format!("{author}: {body}"))
        .layout(
            LayoutStyle::flex_column()
                .height(MSG_ROW_HEIGHT)
                .gap(2.0)
                .padding(6.0),
        )
        .class("w-full rounded-sm bg-surface")
        .child(label(author))
        .child(label(body))
}

fn members_tab(state: ChatState) -> NodeBuilder {
    let idx = state.selected_channel.peek();
    let members: Vec<String> = state
        .channels
        .peek()
        .get(idx)
        .map(|c| c.members.clone())
        .unwrap_or_default();
    let count = members.len();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0))
        .child(label(format!("Members ({count})")))
        .child(
            sigil::widgets::virtual_list(members, |_, name| member_row(name))
                .row_height(MEMBER_ROW_HEIGHT)
                .viewport_height(LIST_VIEWPORT_HEIGHT),
        )
}

fn member_row(name: &str) -> NodeBuilder {
    container()
        .role(Role::ListItem)
        .label(name.to_owned())
        .layout(
            LayoutStyle::flex_row()
                .height(MEMBER_ROW_HEIGHT)
                .padding(4.0)
                .align_items(AlignItems::Center),
        )
        .class("w-full rounded-sm bg-surface")
        .child(label(name.to_owned()))
}

fn pinned_tab(state: ChatState) -> NodeBuilder {
    let idx = state.selected_channel.peek();
    let pinned: Vec<String> = state
        .channels
        .peek()
        .get(idx)
        .map(|c| c.pinned.clone())
        .unwrap_or_default();
    let count = pinned.len();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0))
        .child(label(format!("Pinned ({count})")))
        .child(
            sigil::widgets::virtual_list(pinned, |_, line| pinned_row(line))
                .row_height(PINNED_ROW_HEIGHT)
                .viewport_height(LIST_VIEWPORT_HEIGHT),
        )
}

fn pinned_row(line: &str) -> NodeBuilder {
    container()
        .role(Role::ListItem)
        .label(line.to_owned())
        .layout(
            LayoutStyle::flex_row()
                .height(PINNED_ROW_HEIGHT)
                .padding(6.0)
                .align_items(AlignItems::Center),
        )
        .class("w-full rounded-sm bg-surface")
        .child(label(line.to_owned()))
}
