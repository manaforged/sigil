use std::cell::RefCell;
use std::rc::Rc;

use crate::events::{Key, LogicalKey, MouseButton, NodeEventsExt, Shortcut};
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived};
use crate::style::{FocusRing, NodeStyleExt, StyleAtom, Token, space};

use super::super::drag::{Axis, axis_drag};
use super::super::interaction::InteractionState;
use super::super::patterns::row_states;
use super::chrome::{ContextKind, TableChrome};
use super::columns::{CellSpec, ColAlign, TableColumn, flex_cell};
use super::hscroll::HScroll;
use super::select;
use super::sort_mark::sort_mark;
use super::state::{SortDir, TableState};
use crate::kernel::text;

const RESIZE_HANDLE: f32 = space(2);
const RING_INSET: f32 = -space(1);
const HEADER_WEIGHT: u16 = 500;

pub(super) static HEADER_TEXT: &[StyleAtom] = &[
    StyleAtom::FontWeight(HEADER_WEIGHT),
    Token::OnSurfaceMuted.text(),
];
const REORDER_THRESHOLD: f32 = 12.0;
const REORDER_STEP: f32 = 80.0;

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
                .fixed_size(RESIZE_HANDLE, height)
                .position(Position::Absolute)
                .inset_right(0.0)
                .inset_top(0.0),
        );
    let widths = state.clone();
    axis_drag(
        handle,
        Axis::X,
        move || widths.column_width(column).unwrap_or(160.0),
        move |base, delta| {
            state.pin_column_width(column, base + delta);
        },
        || {},
    )
}

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
    let state = state.clone();
    let chrome = chrome.clone();
    let HeaderSpec {
        height,
        select,
        pad_x,
        disabled,
    } = spec;
    let track = container()
        .layout(track_layout(height))
        .children_reactive(move || {
            let _ = state.column_layout().get();
            let ctx = HeaderCtx {
                state: &state,
                chrome: &chrome,
                select,
                pad_x,
                disabled,
            };
            let mut cells = Vec::new();
            if select {
                cells.push(select::header_cell(ctx.state, height));
            }
            cells.extend(state.visible_columns().into_iter().enumerate().map(
                |(visual, (def_index, column))| {
                    header_cell(&ctx, column, height, visual, def_index)
                },
            ));
            cells
        });
    container()
        .role(Role::Row)
        .row_index(1)
        .layout(header_layout(crate::layout::snap::ruled(height), pad_x))
        .class("border-b")
        .class([Token::TableHeader.bg(), Token::Border.border()])
        .child(hscroll.shifted(track))
}

fn header_layout(height: f32, pad_x: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .padding_x(pad_x)
}

fn track_layout(height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .grow(1.0)
        .height(height)
        .align_items(AlignItems::Center)
}

struct HeaderCtx<'a, Row: 'static> {
    state: &'a TableState<Row>,
    chrome: &'a TableChrome<Row>,
    select: bool,
    pad_x: f32,
    disabled: Option<Signal<bool>>,
}

fn cell_states(cell: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    interaction
        .observe_style(row_states(cell, disabled))
        .focus_ring(FocusRing::new(interaction.focus_visible()).offset(RING_INSET))
}

fn header_cell<Row: 'static>(
    ctx: &HeaderCtx<'_, Row>,
    column: &TableColumn<Row>,
    height: f32,
    visual: usize,
    def_index: usize,
) -> NodeBuilder {
    let layout = ctx.state.column_layout().peek();
    let override_px = layout.widths.get(column.id()).copied();
    let id = column.id();
    let title = header_title(
        column,
        ctx.pad_x,
        sort_mark(ctx.state, id, column.is_sortable()),
    );
    let cell = container()
        .role(Role::ColumnHeader)
        .label(column.title().to_owned())
        .column_index(def_index + 1 + usize::from(ctx.select))
        .hittable(true)
        .layout(flex_cell(CellSpec {
            policy: column.col_width(),
            override_px,
            height,
            clip: false,
            min_px: column.min_px(),
            align: ColAlign::Start,
            pad_x: ctx.pad_x,
        }))
        .child(title)
        .child(resize_handle(ctx.state.clone(), id, height));
    let cell = cell_states(cell, ctx.disabled);
    if column.is_sortable() {
        sortable_cell(ctx.state, id, visual, cell, ctx.chrome.clone())
    } else {
        reorder_only(ctx.state.clone(), id, visual, cell, ctx.chrome.clone())
    }
}

fn header_title<Row: 'static>(
    column: &TableColumn<Row>,
    pad_x: f32,
    mark: NodeBuilder,
) -> NodeBuilder {
    let title = text(column.title().to_owned())
        .nowrap()
        .ellipsize()
        .class("text-xs")
        .class(HEADER_TEXT);
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
        .child(title)
        .child(mark)
}

fn sortable_cell<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
    visual: usize,
    cell: NodeBuilder,
    chrome: TableChrome<Row>,
) -> NodeBuilder {
    let direction = sort_direction(state, column);
    let session = Rc::new(RefCell::new(None::<(f32, usize)>));
    let down = Rc::clone(&session);
    let up = Rc::clone(&session);
    let sort_state = state.clone();
    let move_state = state.clone();
    let node = cell
        .sort_direction(direction)
        .focusable(true)
        .hittable(true)
        .pointer_down(move |event| {
            if event.button == Some(MouseButton::Right) {
                chrome.open(
                    ContextKind::Header(column),
                    (event.window_position.x, event.window_position.y),
                );
                event.stop_propagation();
                return;
            }
            if event.button == Some(MouseButton::Left) {
                *down.borrow_mut() = Some((event.window_position.x, visual));
            }
        })
        .pointer_up(move |event| {
            if event
                .button
                .is_some_and(|button| button != MouseButton::Left)
            {
                return;
            }
            let Some((start_x, from)) = up.borrow_mut().take() else {
                sort_state.toggle_sort(column);
                return;
            };
            let delta = event.window_position.x - start_x;
            if delta.abs() >= REORDER_THRESHOLD {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "values are bounded to the target range before the cast"
                )]
                let steps = (delta / REORDER_STEP).round() as i32;
                move_state.move_column(from, from.saturating_add_signed(steps as isize));
            } else {
                sort_state.toggle_sort(column);
            }
        });
    sort_keys(node, state, column)
}

fn sort_keys<Row: 'static>(
    cell: NodeBuilder,
    state: &TableState<Row>,
    column: &'static str,
) -> NodeBuilder {
    let key_state = state.clone();
    let space_state = state.clone();
    cell.key(Key::Enter, move |event| {
        event.stop_propagation();
        key_state.toggle_sort(column);
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
    let session = Rc::new(RefCell::new(None::<(f32, usize)>));
    let down = Rc::clone(&session);
    cell.pointer_down(move |event| {
        if event.button == Some(MouseButton::Right) {
            chrome.open(
                ContextKind::Header(column),
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
        *down.borrow_mut() = Some((event.position.x, visual));
        event.capture_pointer();
    })
    .pointer_up(move |event| {
        let Some((start_x, from)) = session.borrow_mut().take() else {
            return;
        };
        let delta = event.position.x - start_x;
        if delta.abs() >= REORDER_THRESHOLD {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "values are bounded to the target range before the cast"
            )]
            let steps = (delta / REORDER_STEP).round() as i32;
            state.move_column(from, from.saturating_add_signed(steps as isize));
        }
        event.stop_propagation();
    })
}

fn sort_direction<Row: 'static>(
    state: &TableState<Row>,
    column: &'static str,
) -> Signal<Option<crate::kernel::SortDirection>> {
    let sort = state.sort_key();
    derived(move || {
        sort.get()
            .filter(|key| key.column == column)
            .map(|key| match key.dir {
                SortDir::Ascending => crate::kernel::SortDirection::Ascending,
                SortDir::Descending => crate::kernel::SortDirection::Descending,
            })
    })
}
