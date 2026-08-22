//! Body rows for the `table()` view (DR-057 D2).
//!
//! The row render closure handed to `virtual_list` receives the
//! visible position and the row index (into [`TableState::rows`]) and
//! builds a [`Role::Row`] flex row with one [`Role::GridCell`] per
//! visible column. Selection styling and the AccessKit selected channel
//! read [`TableState::selection`] reactively; hover comes from the
//! `interactive()` wrapper; pointer-down selection mirrors the
//! virtual-list modifier shapes (plain → only, ctrl/cmd → toggle,
//! shift → range). Right-click selects the row and opens the
//! table context menu.

use sigil_events::prelude::{ModifiersState, MouseButton, NodeEventsExt, PointerEvent};
use sigil_kernel::{IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_reactive::derived_eq;
use sigil_style::{Fill, NodeStyleExt, StateVariants};

use super::super::interactive::interactive;
use super::chrome::{ContextKind, TableChrome};
use super::columns::flex_cell;
use super::hscroll::HScroll;
use super::state::TableState;

/// Selected-row class — the accent tint (Radix step 5) so cell text
/// keeps its `on-surface` colour against the highlight. Step 3 read as
/// effectively invisible (1.05:1 against the unselected row); step 5
/// measures 1.28:1 while keeping text contrast at 13.4:1.
const SELECTED_CLASS: &str = "bg-accent-5";

/// Build the row render closure the table's body `virtual_list`
/// binds. `hscroll` shifts each row's cell track by the shared
/// horizontal offset; the row box itself never moves, so its clip and
/// full-width background behave exactly as before.
pub(super) fn body_row<Row: 'static>(
    state: TableState<Row>,
    row_height: f32,
    chrome: TableChrome<Row>,
    hscroll: HScroll,
) -> impl Fn(usize, &u32) -> NodeBuilder {
    move |visible_pos: usize, row_index: &u32| {
        build_row(
            &state,
            row_height,
            &chrome,
            hscroll,
            (visible_pos, *row_index),
        )
    }
}

fn build_row<Row: 'static>(
    state: &TableState<Row>,
    row_height: f32,
    chrome: &TableChrome<Row>,
    hscroll: HScroll,
    slot: (usize, u32),
) -> NodeBuilder {
    let (visible_pos, row_index) = slot;
    let rows = state.rows().peek();
    let Some(row) = rows.get(row_index as usize) else {
        // Stale index mid-mutation (rows replaced while the window
        // still shows the old frame): paint an empty slot; the next
        // reactive pass rebuilds it.
        return container();
    };
    let selection = state.selection();
    let is_selected = derived_eq(move || selection.get().contains(&row_index));
    let state_cells = state.clone();
    // The cell track slides inside the stationary row: the row keeps
    // the x clip (the emit walk's culling depends on it) and its
    // full-width background, while cells travel under the clip.
    let track = container()
        .layout(track_layout(row_height))
        .children_reactive(move || {
            let layout = state_cells.column_layout().get();
            // `get`, not `peek`: this closure has to re-run when the row data
            // changes, or a list that refreshes itself paints its first frame
            // forever.
            let live_rows = state_cells.rows().get();
            let Some(live_row) = live_rows.get(row_index as usize) else {
                return Vec::new();
            };
            state_cells
                .visible_columns()
                .into_iter()
                .map(|(def_index, column)| {
                    let override_px = layout.widths.get(column.id()).copied();
                    body_cell(
                        column.cell_render().map(|render| render(live_row)),
                        def_index,
                        flex_cell(
                            column.col_width(),
                            override_px,
                            row_height,
                            true,
                            column.min_px(),
                        ),
                    )
                })
                .collect::<Vec<_>>()
        });
    let cells = container()
        .role(Role::Row)
        .label(format!("row {}", visible_pos + 1))
        .row_index(visible_pos + 2)
        .selected(is_selected)
        .layout(row_layout(row_height))
        .class("border-b border-gray-6")
        .class_when(is_selected, SELECTED_CLASS)
        .child(hscroll.shifted(track));
    let _ = row;
    interactive(cells)
        .hover(|s| s.bg("surface-hover"))
        .into_node_builder()
        .pointer_down(select_pointer(state.clone(), chrome.clone(), row_index))
}

fn row_layout(height: f32) -> LayoutStyle {
    // Identical to the pre-hscroll row: stationary, viewport-wide,
    // x-clipped. The clip's right edge IS the viewport edge, and the
    // cell track slides underneath it — cells entering from the right
    // live inside the track's extent-floored box, so the emit walk's
    // subtree culling can't drop them.
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .align_items(AlignItems::Center)
        .overflow_x(Overflow::Hidden)
}

fn track_layout(height: f32) -> LayoutStyle {
    // A flex ITEM of the row: grow fills the viewport when content
    // fits (fraction columns stretch exactly as before); the reactive
    // MinWidth atom floors it at the column extent when overflowing,
    // so the shifted track box always spans its cells.
    LayoutStyle::flex_row()
        .grow(1.0)
        .height(height)
        .align_items(AlignItems::Center)
}

fn body_cell(content: Option<Box<dyn IntoNode>>, index: usize, layout: LayoutStyle) -> NodeBuilder {
    container()
        .role(Role::GridCell)
        .column_index(index + 1)
        .layout(layout)
        .child(content.unwrap_or_else(|| Box::new(container())))
}

/// Pointer-down selection with the virtual-list modifier shapes:
/// shift extends the anchored range, ctrl/cmd toggles, anything else
/// selects only this row. Right-click selects (if needed) and opens
/// the context menu.
fn select_pointer<Row: 'static>(
    state: TableState<Row>,
    chrome: TableChrome<Row>,
    row_index: u32,
) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        if event.button == Some(MouseButton::Right) {
            if !state.selection().peek().contains(&row_index) {
                state.select_only(row_index);
            }
            chrome.open(
                ContextKind::Row(row_index),
                (event.window_position.x, event.window_position.y),
            );
            event.stop_propagation();
            return;
        }
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
