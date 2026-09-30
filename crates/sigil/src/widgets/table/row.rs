use std::sync::Arc;

use crate::events::{Key, ModifiersState, MouseButton, NodeEventsExt, PointerEvent};
use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle, Overflow};
use crate::reactive::{Signal, derived};
use crate::style::{StateStyle, StyleAtom, StyleVariant, Token};

use super::super::interaction::InteractionState;
use super::super::patterns::{ROW_SELECTED, ROW_SELECTED_HOVER};
use super::super::shared::{DIMMED, disabled_a11y, reactive_style_state};
use super::chrome::{ContextKind, TableChrome};
use super::columns::{CellSpec, flex_cell};
use super::hscroll::HScroll;
use super::select;
use super::state::{RowEq, TableState};

struct RowValue<Row> {
    rows: Arc<Vec<Row>>,
    index: usize,
    eq: Option<RowEq<Row>>,
}

impl<Row> Clone for RowValue<Row> {
    fn clone(&self) -> Self {
        Self {
            rows: Arc::clone(&self.rows),
            index: self.index,
            eq: self.eq.clone(),
        }
    }
}

impl<Row> PartialEq for RowValue<Row> {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.rows, &other.rows) {
            return true;
        }
        match (self.rows.get(self.index), other.rows.get(other.index)) {
            (None, None) => true,
            (Some(a), Some(b)) => self.eq.as_ref().is_some_and(|eq| eq(a, b)),
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RowSpec {
    pub row_height: f32,
    pub select: bool,
    pub pad_x: f32,
    pub striped: bool,
    pub disabled: Option<Signal<bool>>,
}

static REST: &[StyleAtom] = &[];
static HOVER: &[StyleAtom] = &[Token::SurfaceHover.bg()];
static PRESSED: &[StyleAtom] = &[Token::SurfacePressed.bg()];
static SELECTED: &[StyleAtom] = &[ROW_SELECTED.bg()];
static SELECTED_HOVER: &[StyleAtom] = &[ROW_SELECTED_HOVER.bg()];
static SELECTED_DISABLED: &[StyleAtom] = &[
    ROW_SELECTED.bg(),
    StyleAtom::Opacity(crate::style::DISABLED_OPACITY),
];
static PLAIN_ROW: StyleVariant =
    StyleVariant::new(REST, StateStyle::new(REST, HOVER, HOVER, PRESSED, DIMMED));
static SELECTED_ROW: StyleVariant = StyleVariant::new(
    REST,
    StateStyle::new(
        SELECTED,
        SELECTED_HOVER,
        SELECTED_HOVER,
        SELECTED_HOVER,
        SELECTED_DISABLED,
    ),
);
const STRIPE: Token = Token::SurfaceInset;
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
    if state.rows().peek().get(row_index as usize).is_none() {
        return container();
    }
    let selection = state.selection();
    let is_selected = derived(move || selection.get().contains(&row_index));
    let rows = state.rows();
    let eq = state.row_eq_fn();
    let row_value = derived(move || RowValue {
        rows: rows.get(),
        index: row_index as usize,
        eq: eq.clone(),
    });
    let state_cells = state.clone();
    let track = container()
        .layout(track_layout(row_height))
        .children_reactive(move || row_cells(&state_cells, spec, row_index, &row_value.get()));
    let cells = container()
        .role(Role::Row)
        .label(format!("row {}", visible_pos + 1))
        .row_index(visible_pos + 2)
        .selected(is_selected)
        .layout(row_layout(row_height, spec.pad_x))
        .child(hscroll.shifted(track));
    let mut frame = container()
        .layout(row_layout(row_height, 0.0))
        .class("border-b")
        .class(Token::BorderSubtle.border());
    if spec.striped && visible_pos % 2 == 1 {
        frame = frame.class(STRIPE.bg());
    }
    let cells = row_states(cells, is_selected, spec.disabled).pointer_down(select_pointer(
        state.clone(),
        chrome.clone(),
        (row_index, spec.disabled),
    ));
    frame.child(activate_key(
        cells,
        state,
        chrome,
        (row_index, spec.disabled),
    ))
}

fn activate_key<Row: 'static>(
    row: NodeBuilder,
    state: &TableState<Row>,
    chrome: &TableChrome<Row>,
    target: (u32, Option<Signal<bool>>),
) -> NodeBuilder {
    if chrome.row_activate.is_none() {
        return row;
    }
    let (row_index, disabled) = target;
    let (state, chrome) = (state.clone(), chrome.clone());
    row.focusable(true).key(Key::Enter, move |event| {
        if disabled.is_some_and(|off| off.get()) {
            return;
        }
        chrome.activate(&state, row_index);
        event.stop_propagation();
    })
}

fn row_states(
    row: NodeBuilder,
    selected: Signal<bool>,
    disabled: Option<Signal<bool>>,
) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    let row = disabled_a11y(row, disabled).style_reactive(move || {
        let variant = if selected.get() {
            SELECTED_ROW
        } else {
            PLAIN_ROW
        };
        variant.state(reactive_style_state(interaction, disabled))
    });
    interaction.observe_style(row)
}

fn row_cells<Row: 'static>(
    state: &TableState<Row>,
    spec: RowSpec,
    row_index: u32,
    value: &RowValue<Row>,
) -> Vec<NodeBuilder> {
    let layout = state.column_layout().get();
    let Some(live_row) = value.rows.get(value.index) else {
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
    target: (u32, Option<Signal<bool>>),
) -> impl Fn(&mut PointerEvent) + 'static {
    let (row_index, disabled) = target;
    move |event| {
        if disabled.is_some_and(|off| off.get()) {
            return;
        }
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
        if !(event.modifiers.shift() || event.modifiers.ctrl() || event.modifiers.meta()) {
            chrome.activate(&state, row_index);
        }
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
