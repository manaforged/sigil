//! Board pane — horizontal [`tabs`] across Todo / Doing / Done, each
//! routing to a [`virtual_list`] of cards.

use sigil::prelude::*;

use crate::state::{BoardState, COL_DOING, COL_DONE, COL_TODO, Card, column_title};

const CARD_HEIGHT: f32 = 56.0;
const BOARD_VIEWPORT_HEIGHT: f32 = 520.0;

pub(crate) fn board_pane(state: BoardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0).padding(8.0))
        .class("h-full rounded-md bg-surface-hover")
        .child(
            tabs(
                state.active_tab,
                vec![
                    String::from(column_title(COL_TODO)),
                    String::from(column_title(COL_DOING)),
                    String::from(column_title(COL_DONE)),
                ],
            )
            .label("Board columns")
            .content_for(COL_TODO, column_list(state, COL_TODO))
            .content_for(COL_DOING, column_list(state, COL_DOING))
            .content_for(COL_DONE, column_list(state, COL_DONE)),
        )
}

fn column_list(state: BoardState, col: usize) -> NodeBuilder {
    // Snapshot the column at build time. The widget re-builds when the
    // board structure changes via the surrounding effects + signal
    // graph; for this dogfood pass that snapshot is fine — cards don't
    // animate, and the modal flow rebuilds the tree on commit.
    let cards = state.columns.peek().get(col).cloned().unwrap_or_default();
    let count = cards.len();

    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(4.0).padding(8.0))
        .class("w-full")
        .child(label(format!("{} ({count})", column_title(col))))
        .child(
            sigil::widgets::virtual_list(cards, move |idx, card| card_row(state, col, idx, card))
                .row_height(CARD_HEIGHT)
                .viewport_height(BOARD_VIEWPORT_HEIGHT),
        )
}

fn card_row(state: BoardState, col: usize, idx: usize, card: &Card) -> NodeBuilder {
    let title = card.title.clone();
    let detail = card.description.clone();
    container()
        .role(Role::Button)
        .label(card.title.clone())
        .focusable(true)
        .layout(
            LayoutStyle::flex_column()
                .height(CARD_HEIGHT)
                .gap(2.0)
                .padding(8.0),
        )
        .class("w-full rounded-sm bg-surface")
        .pointer_down(move |ev| {
            state.open_edit(col, idx);
            ev.stop_propagation();
        })
        .child(label(title))
        .child(label(detail))
}
