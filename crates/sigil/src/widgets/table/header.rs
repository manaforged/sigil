use std::cell::Cell;
use std::rc::Rc;

use crate::events::{Key, LogicalKey, MouseButton, NodeEventsExt, PointerEvent, Shortcut};
use crate::kernel::{NodeBuilder, Role, SortDirection, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position, snap};
use crate::reactive::{Signal, derived};
use crate::style::{NodeStyleExt, StyleAtom, Token, space};

use super::super::drag::{Axis, axis_drag};
use super::super::interaction::InteractionState;
use super::super::patterns::row_states;
use super::chrome::{ContextKind, TableChrome};
use super::columns::{ColAlign, TableColumn};
use super::hscroll::HScroll;
use super::select;
use super::sort_mark::sort_mark;
use super::state::{SortDir, TableState};

pub(super) static HEADER_TEXT: &[StyleAtom] =
    &[StyleAtom::FontWeight(500), Token::OnSurfaceMuted.text()];
const REORDER_THRESHOLD: f32 = 12.0;
const REORDER_STEP: f32 = 80.0;

#[derive(Clone, Copy)]
pub(super) struct HeaderSpec {
    pub height: f32,
    pub select: bool,
    pub pad_x: f32,
    pub disabled: Option<Signal<bool>>,
}

pub(super) fn header_row<Row: 'static>(
    state: &TableState<Row>,
    spec: HeaderSpec,
    chrome: &TableChrome<Row>,
    hscroll: HScroll,
) -> NodeBuilder {
    let (state, chrome) = (state.clone(), chrome.clone());
    let track = container()
        .layout(track_layout(spec.height))
        .children_reactive(move || {
            let _ = state.column_layout().get();
            let mut cells = Vec::new();
            if spec.select {
                cells.push(select::header_cell(&state, spec.height));
            }
            cells.extend(state.visible_columns().into_iter().enumerate().map(
                |(visual, (def_index, column))| {
                    header_cell(&state, &chrome, spec, column, (visual, def_index))
                },
            ));
            cells
        });
    container()
        .role(Role::Row)
        .row_index(1)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(snap::ruled(spec.height))
                .shrink(0.0)
                .align_items(AlignItems::Center)
                .padding_x(spec.pad_x),
        )
        .class("border-b")
        .class([Token::TableHeader.bg(), Token::Border.border()])
        .child(hscroll.shifted(track))
}

pub(super) fn track_layout(height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .grow(1.0)
        .height(height)
        .align_items(AlignItems::Center)
}

fn header_cell<Row: 'static>(
    state: &TableState<Row>,
    chrome: &TableChrome<Row>,
    spec: HeaderSpec,
    column: &TableColumn<Row>,
    (visual, def_index): (usize, usize),
) -> NodeBuilder {
    let id = column.id();
    let override_px = state.column_layout().peek().widths.get(id).copied();
    let cell = container()
        .role(Role::ColumnHeader)
        .label(column.title().to_owned())
        .column_index(def_index + 1 + usize::from(spec.select))
        .hittable(true)
        .layout(column.cell_layout(override_px, spec.height, None))
        .child(header_title(
            column,
            spec.pad_x,
            sort_mark(state, id, column.is_sortable()),
        ))
        .child(resize_handle(state.clone(), id, spec.height));
    let interaction = InteractionState::new(true);
    let cell = interaction
        .observe_style(row_states(cell, spec.disabled))
        .focus_ring(interaction.focus_ring().offset(-space(1)));
    if column.is_sortable() {
        sortable_cell(state, id, visual, cell, chrome.clone())
    } else {
        reorder_only(state.clone(), id, visual, cell, chrome.clone())
    }
}

fn resize_handle<Row: 'static>(
    state: TableState<Row>,
    column: &'static str,
    height: f32,
) -> NodeBuilder {
    let handle = container()
        .role(Role::Splitter)
        .label(format!("Resize {column}"))
        .hittable(true)
        .layout(
            LayoutStyle::block()
                .fixed_size(space(2), height)
                .position(Position::Absolute)
                .inset_right(0.0)
                .inset_top(0.0),
        );
    let widths = state.clone();
    axis_drag(
        handle,
        Axis::X,
        move || widths.column_width(column).unwrap_or(160.0),
        move |base, delta| state.pin_column_width(column, base + delta),
        || {},
    )
}

fn header_title<Row: 'static>(
    column: &TableColumn<Row>,
    pad_x: f32,
    mark: NodeBuilder,
) -> NodeBuilder {
    let justify = if column.alignment() == ColAlign::End {
        JustifyContent::End
    } else {
        JustifyContent::Start
    };
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .padding_x(pad_x)
                .align_items(AlignItems::Center)
                .justify_content(justify),
        )
        .child(
            text(column.title().to_owned())
                .nowrap()
                .ellipsize()
                .class("text-xs")
                .class(HEADER_TEXT),
        )
        .child(mark)
}

fn open_menu<Row: 'static>(
    chrome: &TableChrome<Row>,
    column: &'static str,
    event: &mut PointerEvent,
) -> bool {
    if event.button != Some(MouseButton::Right) {
        return false;
    }
    chrome.open(
        ContextKind::Header(column),
        (event.window_position.x, event.window_position.y),
    );
    event.stop_propagation();
    true
}

fn try_reorder<Row: 'static>(state: &TableState<Row>, from: usize, delta: f32) -> bool {
    if delta.abs() < REORDER_THRESHOLD {
        return false;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    let steps = (delta / REORDER_STEP).round() as i32;
    state.move_column(from, from.saturating_add_signed(steps as isize));
    true
}

fn sortable_cell<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
    visual: usize,
    cell: NodeBuilder,
    chrome: TableChrome<Row>,
) -> NodeBuilder {
    let start = Rc::new(Cell::new(None::<f32>));
    let down = Rc::clone(&start);
    let (up_state, enter_state, space_state) = (state.clone(), state.clone(), state.clone());
    cell.sort_direction(state.sort_direction(column))
        .focusable(true)
        .hittable(true)
        .pointer_down(move |event| {
            if !open_menu(&chrome, column, event) && event.button == Some(MouseButton::Left) {
                down.set(Some(event.window_position.x));
            }
        })
        .pointer_up(move |event| {
            if event
                .button
                .is_some_and(|button| button != MouseButton::Left)
            {
                return;
            }
            let moved = start
                .take()
                .is_some_and(|x| try_reorder(&up_state, visual, event.window_position.x - x));
            if !moved {
                up_state.toggle_sort(column);
            }
        })
        .key(Key::Enter, move |event| {
            event.stop_propagation();
            enter_state.toggle_sort(column);
        })
        .key(Shortcut::plain(LogicalKey::character(" ")), move |event| {
            event.stop_propagation();
            space_state.toggle_sort(column);
        })
}

fn reorder_only<Row: 'static>(
    state: TableState<Row>,
    column: &'static str,
    visual: usize,
    cell: NodeBuilder,
    chrome: TableChrome<Row>,
) -> NodeBuilder {
    let start = Rc::new(Cell::new(None::<f32>));
    let down = Rc::clone(&start);
    cell.pointer_down(move |event| {
        if open_menu(&chrome, column, event)
            || event
                .button
                .is_some_and(|button| button != MouseButton::Left)
        {
            return;
        }
        down.set(Some(event.position.x));
        event.capture_pointer();
    })
    .pointer_up(move |event| {
        let Some(x) = start.take() else {
            return;
        };
        try_reorder(&state, visual, event.position.x - x);
        event.stop_propagation();
    })
}

impl<Row: 'static> TableState<Row> {
    fn sort_direction(&self, column: &'static str) -> Signal<Option<SortDirection>> {
        let sort = self.sort_key();
        derived(move || {
            sort.get()
                .filter(|key| key.column == column)
                .map(|key| match key.dir {
                    SortDir::Ascending => SortDirection::Ascending,
                    SortDir::Descending => SortDirection::Descending,
                })
        })
    }
}
