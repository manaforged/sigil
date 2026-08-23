//! Board pane — three columns with drag between them.

use sigil::prelude::*;
use sigil::widgets::{DragDropExt, DragState, drag_ghost};

use crate::state::{BoardState, COL_DOING, COL_DONE, COL_TODO, Card, column_title};

const CARD_HEIGHT: f32 = 56.0;
const BOARD_VIEWPORT_HEIGHT: f32 = 520.0;

pub(crate) fn board_pane(state: BoardState) -> NodeBuilder {
    let drag = DragState::<(usize, usize)>::new();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(8.0).padding(8.0))
        .class(classes!("h-full rounded-md bg-surface-hover"))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0).gap(8.0))
                .child(column_list(state, COL_TODO, drag.clone()))
                .child(column_list(state, COL_DOING, drag.clone()))
                .child(column_list(state, COL_DONE, drag.clone())),
        )
        .child(drag_ghost(drag, |payload| {
            let (from, idx) = *payload;
            text(format!("card {from}:{idx}")).class(classes!("text-sm"))
        }))
}

fn column_list(state: BoardState, col: usize, drag: DragState<(usize, usize)>) -> NodeBuilder {
    let cards = state.columns.peek().get(col).cloned().unwrap_or_default();
    let count = cards.len();
    let drop_state = state;
    let drop_drag = drag.clone();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(4.0).padding(8.0))
        .class(classes!("w-full"))
        .child(label(format!("{} ({count})", column_title(col))))
        .child(
            sigil::widgets::virtual_list(cards, move |idx, card| {
                card_row(state, col, idx, card, drag.clone())
            })
            .row_height(CARD_HEIGHT)
            .viewport_height(BOARD_VIEWPORT_HEIGHT),
        )
        .drop_target(
            drop_drag,
            move |(from, _)| *from != col,
            move |(from, idx)| drop_state.move_card(from, idx, col),
        )
}

fn card_row(
    state: BoardState,
    col: usize,
    idx: usize,
    card: &Card,
    drag: DragState<(usize, usize)>,
) -> NodeBuilder {
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
        .class(classes!("w-full rounded-sm bg-surface"))
        .drag_source(drag, (col, idx))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .align_items(AlignItems::Center)
                        .gap(8.0),
                )
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().gap(2.0).grow(1.0))
                        .child(label(title))
                        .child(label(detail)),
                )
                .child(
                    icon_button("\u{270E}", move || state.open_edit(col, idx)).accessible("Edit"),
                ),
        )
}
