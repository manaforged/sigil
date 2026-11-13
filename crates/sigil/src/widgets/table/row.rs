use crate::layout::Space;
use std::sync::Arc;

use crate::events::{MouseButton, NamedKey, PointerEvent};
use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutStyle, Overflow};
use crate::reactive::{Signal, memo};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_fill_once;
use crate::style::{StateStyle, StyleAtom, StyleVariant, Token};
use crate::widgets::{Layout, Widget};

use super::super::interaction::InteractionState;
use super::super::patterns::{ROW_SELECTED, ROW_SELECTED_HOVER};
use super::super::shared::{DIMMED, disabled_a11y, reactive_style_state};
use super::chrome::{ContextKind, TableChrome};
use super::columns::TableColumn;
use super::edit::{CellEditor, editable_cell};
use super::header::track_layout;
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

const STRIPE: Token = Token::SurfaceInset;
const NUMERIC_CLASS: &str = "tabular-nums";

static REST: &[StyleAtom] = &[];
static HOVER: &[StyleAtom] = &[Token::SurfaceHover.background()];
static PRESSED: &[StyleAtom] = &[Token::SurfacePressed.background()];
static SELECTED: &[StyleAtom] = &[ROW_SELECTED.background()];
static SELECTED_HOVER: &[StyleAtom] = &[ROW_SELECTED_HOVER.background()];
static SELECTED_DISABLED: &[StyleAtom] = &[
    ROW_SELECTED.background(),
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

pub(super) fn body_row<Row: 'static>(
    state: TableState<Row>,
    spec: RowSpec,
    chrome: TableChrome<Row>,
    hscroll: HScroll,
) -> impl Fn(usize, &u32) -> NodeBuilder {
    move |visible_pos: usize, &row_index: &u32| {
        build_row(&state, spec, &chrome, hscroll, (visible_pos, row_index))
    }
}

fn build_row<Row: 'static>(
    state: &TableState<Row>,
    spec: RowSpec,
    chrome: &TableChrome<Row>,
    hscroll: HScroll,
    (visible_pos, row_index): (usize, u32),
) -> NodeBuilder {
    let row_height = spec.row_height;
    if state.rows().peek().get(row_index as usize).is_none() {
        return container();
    }
    let selection = state.selection();
    let is_selected = memo(move || selection.get().contains(&row_index));
    let rows = state.rows();
    let eq = state.row_eq_fn();
    let row_value = memo(move || RowValue {
        rows: rows.get(),
        index: row_index as usize,
        eq: eq.clone(),
    });
    let state_cells = state.clone();
    let editor = chrome.edit;
    let track = container()
        .layout(track_layout(row_height))
        .children_reactive(move || {
            row_cells(&state_cells, (spec, editor), row_index, &row_value.get())
        });
    let cells = container()
        .role(Role::Row)
        .label(theme_fill_once(StringKey::Row, &[&(visible_pos + 1)]))
        .row_index(visible_pos + 2)
        .selected(is_selected)
        .layout(row_layout(row_height, spec.pad_x))
        .child(hscroll.shifted(track));
    let mut frame = container()
        .layout(row_layout(row_height, 0.0))
        .class("border-b")
        .class(Token::BorderSubtle.border());
    if spec.striped && visible_pos % 2 == 1 {
        frame = frame.class(STRIPE.background());
    }
    let target = (row_index, spec.disabled);
    let cells = row_states(cells, is_selected, spec.disabled).pointer_down(select_pointer(
        state.clone(),
        chrome.clone(),
        target,
    ));
    frame.child(activate_key(cells, state, chrome, target))
}

fn activate_key<Row: 'static>(
    row: NodeBuilder,
    state: &TableState<Row>,
    chrome: &TableChrome<Row>,
    target: (u32, Option<Signal<bool>>),
) -> NodeBuilder {
    let editable = state.columns().iter().any(TableColumn::is_editable);
    if chrome.row_activate.is_none() && !editable {
        return row;
    }
    let (row_index, disabled) = target;
    let row = if editable {
        row.focus_when(chrome.edit.refocus_trigger(row_index))
    } else {
        row
    };
    let (state, chrome) = (state.clone(), chrome.clone());
    row.focusable(true).key(NamedKey::Enter, move |event| {
        if disabled.is_some_and(|off| off.get()) {
            return;
        }
        if !chrome.edit.begin_row(&state, row_index) {
            chrome.activate(&state, row_index);
        }
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
    (spec, editor): (RowSpec, CellEditor),
    row_index: u32,
    value: &RowValue<Row>,
) -> Vec<NodeBuilder> {
    let layout = state.column_layout().get();
    if value.rows.get(value.index).is_none() {
        return Vec::new();
    }
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
                let content =
                    cell_content(state, column, (spec, editor), (row_index, def_index), value);
                let cell = container()
                    .role(Role::GridCell)
                    .column_index(def_index + offset + 1)
                    .layout(column.cell_layout(
                        layout.widths.get(column.id()).copied(),
                        spec.row_height,
                        Some(spec.pad_x),
                    ))
                    .child(content);
                if column.is_numeric() {
                    cell.class(NUMERIC_CLASS)
                } else {
                    cell
                }
            }),
    );
    cells
}

fn cell_content<Row: 'static>(
    state: &TableState<Row>,
    column: &TableColumn<Row>,
    (spec, editor): (RowSpec, CellEditor),
    cell: (u32, usize),
    value: &RowValue<Row>,
) -> Box<dyn IntoNode> {
    let render = column.cell_render().cloned();
    let rows = Arc::clone(&value.rows);
    let index = value.index;
    let content = move || -> Box<dyn IntoNode> {
        render.as_ref().zip(rows.get(index)).map_or_else(
            || -> Box<dyn IntoNode> { Box::new(container()) },
            |(render, row)| render(row),
        )
    };
    if column.is_editable() {
        Box::new(editable_cell(
            state,
            editor,
            cell,
            spec.disabled,
            move || container().child(content()),
        ))
    } else {
        content()
    }
}

fn row_layout(height: f32, pad_x: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .align_items(AlignItems::Center)
        .padding_x(Space::px(pad_x))
        .overflow_x(Overflow::Hidden)
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
                state.select(row_index);
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
        let modifiers = event.modifiers;
        if modifiers.shift() {
            state.extend(row_index);
        } else if modifiers.ctrl() || modifiers.meta() {
            state.toggle(row_index);
        } else {
            state.select(row_index);
            chrome.activate(&state, row_index);
        }
        event.stop_propagation();
    }
}
