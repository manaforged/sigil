//! Tier-5 kitchen-sink kanban app.
//!
//! Exercises all five tier-5 widgets together: a left sidebar
//! [`virtual_list`] of projects, a vertical [`splitter`] separating
//! sidebar from board pane, a horizontal [`tabs`] strip routing the
//! board pane to Todo / Doing / Done columns (each its own
//! [`virtual_list`] of cards), and a [`modal`] for new/edit-card with
//! two [`text_field`]s for title and description.

#![allow(clippy::similar_names, clippy::assigning_clones)]

mod board;
mod edit_modal;
mod sidebar;
mod state;

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

use sigil::prelude::*;

use crate::board::board_pane;
use crate::edit_modal::edit_modal;
use crate::sidebar::sidebar_pane;
use crate::state::BoardState;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - kanban", 1100, 720), app_tree)
}

/// Build the full app tree under `cx`. Exposed `pub(crate)` so tests
/// (and any future host that wants to embed the demo without `launch`)
/// can call it directly with a fresh `Cx`.
pub(crate) fn app_tree(cx: &mut Cx) -> NodeId {
    let state = BoardState::seed();
    cx.add(root_shell(state))
}

fn root_shell(state: BoardState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Kanban board")
        .layout(LayoutStyle::flex_column().gap(8.0))
        .class(classes!("w-full h-full p-4 bg-surface"))
        .child(top_bar(state))
        .child(work_area(state))
        .child(status_bar(state))
        .child(edit_modal(state))
}

fn top_bar(state: BoardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(48.0)
                .gap(12.0)
                .align_items(AlignItems::Center)
                .padding(8.0),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label("Kanban"))
        .child(
            button("New card")
                .primary()
                .activate(move || state.open_new_card()),
        )
}

fn work_area(state: BoardState) -> NodeBuilder {
    // Vertical (splitter laid out flex-column means rows; horizontal
    // means left/right). The splitter widget's `Horizontal` is the
    // left/right split we want.
    container()
        .layout(LayoutStyle::flex_row().grow(1.0))
        .class(classes!("w-full"))
        .child(
            sigil::widgets::splitter(sigil::widgets::SplitterOrientation::Horizontal)
                .child(sidebar_pane(state), 220.0)
                .min_size(0, 160.0)
                .max_size(0, 400.0)
                .child(board_pane(state), 720.0)
                .min_size(1, 320.0)
                .sizes_bind(state.split_sizes),
        )
}

fn status_bar(state: BoardState) -> NodeBuilder {
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
