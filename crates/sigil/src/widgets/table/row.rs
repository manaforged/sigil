//! Body rows for the `table()` view (DR-057 D2).
//!
//! The row render closure handed to `virtual_list` receives the
//! visible position and the row index (into [`TableState::rows`]) and
//! builds a [`Role::Row`] flex row with one [`Role::GridCell`] per
//! column. Selection styling and the AccessKit selected channel read
//! [`TableState::selection`] reactively; hover comes from the
//! `interactive()` wrapper; pointer-down selection mirrors the
//! virtual-list modifier shapes (plain → only, ctrl/cmd → toggle,
//! shift → range).

use std::rc::Rc;

use sigil_events::prelude::{ModifiersState, MouseButton, NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::derived_eq;
use sigil_style::{Fill, NodeStyleExt, StateVariants};

use super::super::interactive::interactive;
use super::state::TableState;

/// Selected-row class — the soft accent tint (Radix step 3) so cell
/// text keeps its `on-surface` colour against the highlight.
const SELECTED_CLASS: &str = "bg-accent-5";

/// Build the row render closure the table's body `virtual_list`
/// binds. `widths` is the resolved per-column pixel vector shared with
/// the header.
pub(super) fn body_row<Row: 'static>(
    state: TableState<Row>,
    widths: Rc<Vec<f32>>,
    row_height: f32,
) -> impl Fn(usize, &u32) -> NodeBuilder {
    move |visible_pos: usize, row_index: &u32| {
        build_row(&state, &widths, row_height, visible_pos, *row_index)
    }
}

fn build_row<Row: 'static>(
    state: &TableState<Row>,
    widths: &Rc<Vec<f32>>,
    row_height: f32,
    visible_pos: usize,
    row_index: u32,
) -> NodeBuilder {
    let rows = state.rows().peek();
    let Some(row) = rows.get(row_index as usize) else {
        // Stale index mid-mutation (rows replaced while the window
        // still shows the old frame): paint an empty slot; the next
        // reactive pass rebuilds it.
        return container();
    };
    let selection = state.selection();
    let is_selected = derived_eq(move || selection.get().contains(&row_index));
    let mut cells = container()
        .role(Role::Row)
        .label(format!("row {}", visible_pos + 1))
        .row_index(visible_pos + 2)
        .selected(is_selected)
        .layout(row_layout(widths.iter().sum(), row_height))
        .class_when(is_selected, SELECTED_CLASS);
    for (index, (column, width)) in state.columns().iter().zip(widths.iter()).enumerate() {
        cells = cells.child(body_cell(
            column.cell_render().map(|render| render(row)),
            index,
            *width,
        ));
    }
    interactive(cells)
        .hover(|s| s.bg("surface-hover"))
        .into_node_builder()
        .pointer_down(select_pointer(state.clone(), row_index))
}

fn row_layout(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(width, height)
        .align_items(AlignItems::Center)
}

fn body_cell(content: Option<NodeBuilder>, index: usize, width: f32) -> NodeBuilder {
    container()
        .role(Role::GridCell)
        .column_index(index + 1)
        .layout(LayoutStyle::block().width(width).padding_x(8.0))
        .child(content.unwrap_or_else(container))
}

/// Pointer-down selection with the virtual-list modifier shapes:
/// shift extends the anchored range, ctrl/cmd toggles, anything else
/// selects only this row. Non-left buttons pass through untouched.
fn select_pointer<Row: 'static>(
    state: TableState<Row>,
    row_index: u32,
) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        if event
            .button
            .is_some_and(|button| button != MouseButton::Left)
        {
            return;
        }
        apply_click(&state, row_index, event.modifiers);
        event.stop_propagation();
    }
}

fn apply_click<Row: 'static>(state: &TableState<Row>, row_index: u32, modifiers: ModifiersState) {
    if modifiers.shift() {
        state.select_range_to(row_index);
    } else if modifiers.ctrl() || modifiers.meta() {
        state.toggle_selected(row_index);
    } else {
        state.select_only(row_index);
    }
}
