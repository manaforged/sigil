//! Sticky header row for the `table()` view (DR-057 D2).
//!
//! One [`Role::ColumnHeader`] cell per visible column, sharing the
//! column layout signal with the body so header and cells stay aligned.
//! Sortable columns cycle [`TableState::toggle_sort`] on click; a
//! resize handle on the trailing edge pins a pixel width; a drag past
//! the threshold reorders columns; right-click opens the column menu.

use std::cell::RefCell;
use std::rc::Rc;

use sigil_events::prelude::{Key, LogicalKey, MouseButton, NodeEventsExt, Shortcut};
use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived};
use sigil_style::{Fill, NodeStyleExt, StateVariants};

use super::super::drag::{Axis, axis_drag};
use super::super::interactive::interactive;
use super::super::shared::label;
use super::chrome::{ContextKind, TableChrome};
use super::columns::{TableColumn, flex_cell};
use super::hscroll::HScroll;
use super::state::{SortDir, TableState};

const RESIZE_HANDLE: f32 = 8.0;
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
                .shrink(0.0),
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

/// Build the header row. Cells rebuild when the column layout changes.
/// The cell track slides horizontally inside the stationary header
/// (which keeps the full-width background and bottom border), in
/// lockstep with the body rows via the shared `hscroll` offset.
pub(super) fn header_row<Row: 'static>(
    state: &TableState<Row>,
    height: f32,
    chrome: &TableChrome<Row>,
    hscroll: HScroll,
) -> NodeBuilder {
    let state = state.clone();
    let chrome = chrome.clone();
    let track = container()
        .layout(track_layout(height))
        .children_reactive(move || {
            let _ = state.column_layout().get();
            state
                .visible_columns()
                .into_iter()
                .enumerate()
                .map(|(visual, (def_index, column))| {
                    header_cell(
                        &HeaderCtx {
                            state: &state,
                            chrome: &chrome,
                        },
                        column,
                        height,
                        visual,
                        def_index,
                    )
                })
                .collect::<Vec<_>>()
        });
    container()
        .role(Role::Row)
        .row_index(1)
        .layout(header_layout(height))
        .class("bg-surface border-b border-gray-6")
        .child(hscroll.shifted(track))
}

fn header_layout(height: f32) -> LayoutStyle {
    // The header keeps its height. A filled viewport grows the body, and a
    // shrinkable header would give that space up and slide under row one.
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .shrink(0.0)
        .align_items(AlignItems::Center)
}

fn track_layout(height: f32) -> LayoutStyle {
    // Flex item of the header: grow fills the viewport when content
    // fits; the reactive MinWidth atom floors it at the column extent
    // when overflowing, so the shifted track box always spans its
    // cells (the emit walk culls subtrees whose box misses the clip).
    LayoutStyle::flex_row()
        .grow(1.0)
        .height(height)
        .align_items(AlignItems::Center)
}

/// Table context shared by the header cell builders.
struct HeaderCtx<'a, Row: 'static> {
    state: &'a TableState<Row>,
    chrome: &'a TableChrome<Row>,
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
    let cell = container()
        .role(Role::ColumnHeader)
        .label(column.title().to_owned())
        .column_index(def_index + 1)
        .hittable(true)
        .layout(flex_cell(
            column.col_width(),
            override_px,
            height,
            false,
            column.min_px(),
        ))
        .child(
            label(column.title().to_owned())
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).padding_x(8.0))
                .class("text-xs font-medium text-gray-11"),
        )
        .child(text(sort_indicator(ctx.state, id)).class("text-xs text-gray-11"))
        .child(resize_handle(ctx.state.clone(), id, height));
    if column.is_sortable() {
        sortable_cell(ctx.state, id, visual, cell, ctx.chrome.clone())
    } else {
        reorder_only(ctx.state.clone(), id, visual, cell, ctx.chrome.clone())
    }
}

/// Click cycles sort; a drag on the trailing edge resizes; a longer
/// drag reorders.
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
    let node = interactive(cell)
        .hover(|s| s.bg("surface-pressed"))
        .into_node_builder()
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
                // A drag delta spans the window, never the i32 range.
                #[allow(clippy::cast_possible_truncation)]
                let steps = (delta / REORDER_STEP).round() as i32;
                move_state.move_column(from, from.saturating_add_signed(steps as isize));
            } else {
                sort_state.toggle_sort(column);
            }
        });
    sort_keys(node, state, column)
}

/// Wire Enter + Space sort toggles onto a header cell.
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
    interactive(cell)
        .hover(|s| s.bg("surface-pressed"))
        .into_node_builder()
        .pointer_down(move |event| {
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
                // A drag delta spans the window, never the i32 range.
                #[allow(clippy::cast_possible_truncation)]
                let steps = (delta / REORDER_STEP).round() as i32;
                state.move_column(from, from.saturating_add_signed(steps as isize));
            }
            event.stop_propagation();
        })
}

/// ▲/▼/none text for one column, tracked off the sort signal.
fn sort_indicator<Row: 'static>(state: &TableState<Row>, column: &'static str) -> Signal<String> {
    let sort = state.sort_key();
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
) -> Signal<Option<sigil_kernel::SortDirection>> {
    let sort = state.sort_key();
    derived(move || {
        sort.get()
            .filter(|key| key.column == column)
            .map(|key| match key.dir {
                SortDir::Ascending => sigil_kernel::SortDirection::Ascending,
                SortDir::Descending => sigil_kernel::SortDirection::Descending,
            })
    })
}
