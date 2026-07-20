//! View construction for the CSV viewer.
//!
//! The whole table is `table(state)`: the sticky header, sort cycling,
//! windowed body, row hover, and selection all come from the widget.
//! What remains here is the frame — title bar, the configured table,
//! and a status bar reading the engine's selection signal.

use sigil::prelude::*;
use sigil::reactive::derived;

use crate::data::Row;
use crate::state::AppState;

const ROW_HEIGHT: f32 = 24.0;
const HEADER_HEIGHT: f32 = 28.0;
const VIEWPORT_WIDTH: f32 = 928.0;
const VIEWPORT_HEIGHT: f32 = 520.0;

/// Build the full CSV-viewer tree.
pub(crate) fn app_tree(state: &AppState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil CSV viewer")
        .layout(LayoutStyle::flex_column().fixed_size(960.0, 640.0).gap(8.0))
        .class(classes!("p-4 bg-surface"))
        .child(title_bar(state))
        .child(body(state))
        .child(footer(state))
}

fn title_bar(state: &AppState) -> NodeBuilder {
    let total = state.table.rows().peek().len();
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .gap(16.0)
                .align_items(AlignItems::Center)
                .padding(8.0),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label("CSV viewer").class(classes!("text-on-surface font-bold")))
        .child(label(format!("{total} rows")).class(classes!("text-on-surface")))
}

fn body(state: &AppState) -> TableBuilder<Row> {
    table(state.table.clone())
        .row_height(ROW_HEIGHT)
        .header_height(HEADER_HEIGHT)
        .viewport_width(VIEWPORT_WIDTH)
        .viewport_height(VIEWPORT_HEIGHT)
}

fn footer(state: &AppState) -> NodeBuilder {
    let table = state.table.clone();
    let selection_label = derived(move || {
        table.selection().get().iter().next().map_or_else(
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
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label("Click any row to select; click a header to sort"))
        .child(label(selection_label))
}
