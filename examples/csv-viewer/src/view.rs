//! View construction for the CSV viewer.
//!
//! Composes a title bar, a fixed-column header row, a virtualised body
//! (one row per CSV record), and a status bar showing the current
//! selection. The body uses `virtual_list` with a uniform row height;
//! the header is a sibling container so it stays pinned while the
//! body scrolls.

use sigil::prelude::*;
use sigil::reactive::derived;
use sigil::widgets::{SelectionMode, virtual_list};

use crate::data::{HEADERS, Row};
use crate::state::AppState;

const ROW_HEIGHT: f32 = 24.0;
const HEADER_HEIGHT: f32 = 28.0;
const VIEWPORT_WIDTH: f32 = 928.0;
const VIEWPORT_HEIGHT: f32 = 520.0;

/// Logical pixel width allocated to each column. The id and ticker
/// columns are widest so the seven-digit ids never wrap; the
/// numerics get medium widths; the venue tail mops up the rest.
const COLUMN_WIDTHS: [f32; 6] = [96.0, 96.0, 80.0, 96.0, 128.0, 432.0];

/// Build the full CSV-viewer tree.
pub(crate) fn app_tree(state: AppState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil CSV viewer")
        .layout(LayoutStyle::flex_column().fixed_size(960.0, 640.0).gap(8.0))
        .class("p-4 bg-surface")
        .child(title_bar(state))
        .child(header_row())
        .child(body(state))
        .child(footer(state))
}

fn title_bar(state: AppState) -> NodeBuilder {
    let total = state.rows.len();
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .gap(16.0)
                .align_items(AlignItems::Center)
                .padding(8.0),
        )
        .class("w-full rounded-md bg-surface-hover")
        .child(label("CSV viewer").class("text-on-surface font-bold"))
        .child(label(format!("{total} rows")).class("text-on-surface"))
}

fn header_row() -> NodeBuilder {
    let mut row = container()
        .role(Role::Row)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(VIEWPORT_WIDTH, HEADER_HEIGHT)
                .gap(8.0)
                .align_items(AlignItems::Center)
                .padding(4.0),
        )
        .class("w-full rounded-md bg-surface-hover");
    for (col, header) in HEADERS.iter().enumerate() {
        row = row.child(
            container()
                .layout(LayoutStyle::block().width(column_width(col)))
                .child(label(*header).class("text-on-surface font-bold")),
        );
    }
    row
}

fn body(state: AppState) -> NodeBuilder {
    let selected = state.selected;
    let rows: Vec<Row> = state.rows.to_vec();
    let list = virtual_list(rows, move |index, row: &Row| body_row(index, row, selected))
        .row_height(ROW_HEIGHT)
        .viewport_height(VIEWPORT_HEIGHT)
        .viewport_width(VIEWPORT_WIDTH)
        .selection(SelectionMode::Single)
        .select(move |index| selected.set(Some(index)));
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(VIEWPORT_WIDTH, VIEWPORT_HEIGHT)
                .grow(1.0),
        )
        .child(list)
}

fn body_row(index: usize, row: &Row, selected: Signal<Option<usize>>) -> NodeBuilder {
    let mut cells = container()
        .role(Role::Row)
        .label(format!("row {}", index + 1))
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(VIEWPORT_WIDTH, ROW_HEIGHT)
                .gap(8.0)
                .align_items(AlignItems::Center)
                .padding(4.0),
        )
        .class("text-on-surface")
        .pointer_down(move |_| selected.set(Some(index)));
    for col in 0..HEADERS.len() {
        let text_value = row.cell(col).to_owned();
        cells = cells.child(
            container()
                .layout(LayoutStyle::block().width(column_width(col)))
                .child(label(text_value)),
        );
    }
    cells
}

fn footer(state: AppState) -> NodeBuilder {
    let selected = state.selected;
    let selection_label = derived(move || {
        selected.get().map_or_else(
            || String::from("No selection"),
            |index| format!("Selected row #{}", index + 1),
        )
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .gap(16.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .padding(8.0),
        )
        .class("w-full rounded-md bg-surface-hover")
        .child(label("Click any row to select"))
        .child(label(selection_label))
}

fn column_width(col: usize) -> f32 {
    COLUMN_WIDTHS.get(col).copied().unwrap_or(96.0)
}
