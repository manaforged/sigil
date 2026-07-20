//! Sticky header row for the `table()` view (DR-057 D2).
//!
//! One [`Role::ColumnHeader`] cell per column, sharing the resolved
//! width vector with the body rows so header and cells stay aligned.
//! Sortable columns are interactive: pointer release or Enter/Space
//! cycles [`TableState::toggle_sort`], a ▲/▼ indicator tracks
//! [`TableState::sort`], and the AccessKit sort direction follows
//! reactively.

use sigil_events::prelude::{Key, LogicalKey, NodeEventsExt, Shortcut};
use sigil_kernel::{NodeBuilder, Role, SortDirection, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived};
use sigil_style::{Fill, NodeStyleExt, StateVariants};

use super::super::interactive::interactive;
use super::super::shared::label;
use super::columns::TableColumn;
use super::state::{SortDir, TableState};

/// Build the header row. `widths` is the resolved per-column pixel
/// vector the body rows also use; `height` is the row height.
pub(super) fn header_row<Row: 'static>(
    state: &TableState<Row>,
    widths: &[f32],
    height: f32,
) -> NodeBuilder {
    let mut row = container()
        .role(Role::Row)
        .row_index(1)
        .layout(header_layout(widths.iter().sum(), height))
        .class("bg-surface-hover");
    for (index, (column, width)) in state.columns().iter().zip(widths).enumerate() {
        row = row.child(header_cell(state, column, *width, height, index));
    }
    row
}

fn header_layout(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(width, height)
        .align_items(AlignItems::Center)
}

fn header_cell<Row: 'static>(
    state: &TableState<Row>,
    column: &TableColumn<Row>,
    width: f32,
    height: f32,
    index: usize,
) -> NodeBuilder {
    let cell = container()
        .role(Role::ColumnHeader)
        .label(column.title().to_owned())
        .column_index(index + 1)
        .layout(cell_layout(width, height))
        .child(label(column.title().to_owned()).class("text-on-surface font-bold"))
        .child(text(sort_indicator(state, column.id())).class("text-on-surface"));
    if !column.is_sortable() {
        return cell;
    }
    sortable_cell(state.clone(), column.id(), cell)
}

fn cell_layout(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(width, height)
        .gap(4.0)
        .align_items(AlignItems::Center)
        .padding_x(8.0)
}

/// ▲/▼/none text for one column, tracked off the sort signal.
fn sort_indicator<Row: 'static>(state: &TableState<Row>, column: &'static str) -> Signal<String> {
    let sort = state.sort();
    derived(move || {
        let glyph = match sort.get() {
            Some(key) if key.column == column => match key.dir {
                SortDir::Ascending => "▲",
                SortDir::Descending => "▼",
            },
            _ => "",
        };
        glyph.to_owned()
    })
}

/// AccessKit sort direction for one column, tracked off the sort
/// signal. `None` while another column (or no column) is sorted.
fn sort_direction<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
) -> Signal<Option<SortDirection>> {
    let sort = state.sort();
    derived(move || {
        sort.get()
            .filter(|key| key.column == column)
            .map(|key| match key.dir {
                SortDir::Ascending => SortDirection::Ascending,
                SortDir::Descending => SortDirection::Descending,
            })
    })
}

/// Wrap a sortable column's cell with interaction: hover styling,
/// pointer-release + Enter/Space activation, and the reactive
/// AccessKit sort-direction channel.
fn sortable_cell<Row: 'static>(
    state: TableState<Row>,
    column: &'static str,
    cell: NodeBuilder,
) -> NodeBuilder {
    let direction = sort_direction(&state, column);
    let toggle = move || state.toggle_sort(column);
    let pointer_toggle = toggle.clone();
    let enter_toggle = toggle.clone();
    interactive(cell)
        .hover(|s| s.bg("surface-pressed"))
        .into_node_builder()
        .sort_direction(direction)
        .focusable(true)
        .pointer_up(move |_| pointer_toggle())
        .key(Key::Enter, move |event| {
            event.stop_propagation();
            enter_toggle();
        })
        .key(Shortcut::plain(LogicalKey::character(" ")), move |event| {
            event.stop_propagation();
            toggle();
        })
}
