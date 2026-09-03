use sigil_events::prelude::{ModifiersState, MouseButton, NodeEventsExt, PointerEvent};
use sigil_kernel::{IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_reactive::derived_eq;
use sigil_style::{Fill, NodeStyleExt, StateVariants};

use super::super::interactive::interactive;
use super::chrome::{ContextKind, TableChrome};
use super::columns::{CellSpec, flex_cell};
use super::hscroll::HScroll;
use super::select;
use super::state::TableState;

#[derive(Clone, Copy)]
pub(super) struct RowSpec {
    pub row_height: f32,
    pub select: bool,
    pub pad_x: f32,
    pub striped: bool,
}

const SELECTED_CLASS: &str = "bg-accent-5";
const STRIPE_CLASS: &str = "bg-surface-0";
const NUMERIC_CLASS: &str = "tabular-nums";

pub(super) fn body_row<Row: 'static>(
    state: TableState<Row>,
    spec: RowSpec,
    chrome: TableChrome<Row>,
    hscroll: HScroll,
) -> impl Fn(usize, &u32) -> NodeBuilder {
    move |visible_pos: usize, row_index: &u32| {
        build_row(&state, spec, &chrome, hscroll, (visible_pos, *row_index))
    }
}

fn build_row<Row: 'static>(
    state: &TableState<Row>,
    spec: RowSpec,
    chrome: &TableChrome<Row>,
    hscroll: HScroll,
    slot: (usize, u32),
) -> NodeBuilder {
    let (visible_pos, row_index) = slot;
    let row_height = spec.row_height;
    let rows = state.rows().peek();
    let Some(row) = rows.get(row_index as usize) else {
        return container();
    };
    let selection = state.selection();
    let is_selected = derived_eq(move || selection.get().contains(&row_index));
    let state_cells = state.clone();
    let track = container()
        .layout(track_layout(row_height))
        .children_reactive(move || row_cells(&state_cells, spec, row_index));
    let mut cells = container()
        .role(Role::Row)
        .label(format!("row {}", visible_pos + 1))
        .row_index(visible_pos + 2)
        .selected(is_selected)
        .layout(row_layout(row_height, spec.pad_x))
        .class("border-b border-line");
    if spec.striped && visible_pos % 2 == 1 {
        cells = cells.class(STRIPE_CLASS);
    }
    let cells = cells
        .class_when(is_selected, SELECTED_CLASS)
        .child(hscroll.shifted(track));
    let _ = row;
    interactive(cells)
        .hover(|s| s.bg("surface-hover"))
        .into_node_builder()
        .pointer_down(select_pointer(state.clone(), chrome.clone(), row_index))
}

fn row_cells<Row: 'static>(
    state: &TableState<Row>,
    spec: RowSpec,
    row_index: u32,
) -> Vec<NodeBuilder> {
    let layout = state.column_layout().get();
    let live_rows = state.rows().get();
    let Some(live_row) = live_rows.get(row_index as usize) else {
        return Vec::new();
    };
    let offset = usize::from(spec.select);
    let mut cells = Vec::new();
    if spec.select {
        cells.push(select::row_cell(state, row_index, spec.row_height));
    }
    cells.extend(
        state
            .visible_columns()
            .into_iter()
            .map(|(def_index, column)| {
                let override_px = layout.widths.get(column.id()).copied();
                let cell = body_cell(
                    column.cell_render().map(|render| render(live_row)),
                    def_index + offset,
                    flex_cell(CellSpec {
                        policy: column.col_width(),
                        override_px,
                        height: spec.row_height,
                        clip: true,
                        min_px: column.min_px(),
                        align: column.alignment(),
                        pad_x: spec.pad_x,
                    }),
                );
                if column.is_numeric() {
                    cell.class(NUMERIC_CLASS)
                } else {
                    cell
                }
            }),
    );
    cells
}

fn row_layout(height: f32, pad_x: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .align_items(AlignItems::Center)
        .padding_x(pad_x)
        .overflow_x(Overflow::Hidden)
}

fn track_layout(height: f32) -> LayoutStyle {
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
