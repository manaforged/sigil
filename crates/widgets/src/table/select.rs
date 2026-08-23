//! Leading checkbox selection column for the `table()` view, opted
//! into via `TableBuilder::selectable_checkboxes`.
//!
//! A narrow fixed column prepended to the header and every body-row
//! cell track. The row checkbox toggles that row through
//! [`TableState::toggle_selected`]; the header checkbox selects every
//! visible row (or deselects them when all are selected) and paints a
//! dash while the selection is partial. Checkbox clicks stop
//! pointer-down propagation so the row's own pointer selection never
//! fires alongside the toggle. The box art mirrors the `checkbox`
//! widget's idiom (18 px box, stroked white vector mark).

use std::rc::Rc;

use sigil_events::prelude::{MouseButton, NodeEventsExt};
use sigil_kernel::{AccessibleName, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{NodeStyleExt, Rgba8, StyleAtom};

use super::state::TableState;

/// Column width in logical pixels. Counts into the hscroll extent as
/// a fixed column when the opt-in is on. 32 matches the reference
/// DataTable's `w-8` checkbox cell.
pub(super) const WIDTH: f32 = 32.0;
const BOX: f32 = 18.0;
const MARK: f32 = 12.0;

// Same tokens the checkbox widget paints: accent fill when checked
// (or indeterminate), inset surface when empty.
static CHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static UNCHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-inset")];

/// Header-box tri-state over selection ∩ visible rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mark {
    Empty,
    Partial,
    Full,
}

/// The header cell: a select-all-visible checkbox with a partial dash.
pub(super) fn header_cell<Row: 'static>(state: &TableState<Row>, height: f32) -> NodeBuilder {
    let mark = header_mark(state.clone());
    let toggle = state.clone();
    container()
        .role(Role::ColumnHeader)
        .label("Select")
        .column_index(1)
        .layout(cell_layout(height))
        .child(select_box(mark, "Select all visible rows", move || {
            toggle_all(&toggle);
        }))
}

/// One body-row cell: a checkbox bound to that row's selection bit.
pub(super) fn row_cell<Row: 'static>(
    state: &TableState<Row>,
    row: u32,
    height: f32,
) -> NodeBuilder {
    let selection = state.selection();
    let mark = derived_eq(move || {
        if selection.get().contains(&row) {
            Mark::Full
        } else {
            Mark::Empty
        }
    });
    let toggle = state.clone();
    container()
        .role(Role::GridCell)
        .column_index(1)
        .layout(cell_layout(height))
        .child(select_box(
            mark,
            format!("Select row {}", row + 1),
            move || {
                toggle.toggle_selected(row);
            },
        ))
}

/// The clickable box. Left-button down stops propagation so the row's
/// pointer-down selection does not also fire; left-button up toggles
/// exactly once. Other buttons propagate (right-click still opens the
/// row context menu).
fn select_box(
    mark: Signal<Mark>,
    label: impl Into<AccessibleName>,
    toggle: impl Fn() + 'static,
) -> NodeBuilder {
    let checked = derived_eq(move || mark.get() == Mark::Full);
    container()
        .role(Role::CheckBox)
        .label(label)
        .checked(checked)
        .hittable(true)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(BOX, BOX)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class("rounded-full")
        .class("border")
        .class("border-border")
        .style_reactive(move || {
            if mark.get() == Mark::Empty {
                UNCHECKED
            } else {
                CHECKED
            }
        })
        .children_reactive(move || match mark.get() {
            Mark::Full => vec![mark_node(check_path())],
            Mark::Partial => vec![mark_node(dash_path())],
            Mark::Empty => Vec::new(),
        })
        .pointer_down(|event| {
            if event.button == Some(MouseButton::Left) {
                event.stop_propagation();
            }
        })
        .pointer_up(move |event| {
            // Synthetic up events carry no button; real ones carry the
            // released button. Either way only a left release toggles.
            if event
                .button
                .is_none_or(|button| button == MouseButton::Left)
            {
                toggle();
                event.stop_propagation();
            }
        })
}

fn cell_layout(height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(WIDTH, height)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

/// Tri-state for the header box: none of the visible rows selected,
/// some, or all.
fn header_mark<Row: 'static>(state: TableState<Row>) -> Signal<Mark> {
    derived_eq(move || {
        let visible = state.visible().get();
        let selection = state.selection().get();
        let hit = visible.iter().filter(|row| selection.contains(row)).count();
        if hit == 0 {
            Mark::Empty
        } else if hit == visible.len() {
            Mark::Full
        } else {
            Mark::Partial
        }
    })
}

/// Select every visible row, or deselect the visible rows when all of
/// them are selected. Rows hidden by the quick-filter keep their bit.
fn toggle_all<Row: 'static>(state: &TableState<Row>) {
    let visible = state.visible().peek();
    let mut set = (*state.selection().peek()).clone();
    if !visible.is_empty() && visible.iter().all(|row| set.contains(row)) {
        for row in visible.iter() {
            set.remove(row);
        }
    } else {
        set.extend(visible.iter().copied());
    }
    state.selection().set(Rc::new(set));
}

fn mark_node(path: super::super::vector::PathNode) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(MARK, MARK))
        .child(path)
}

// Stroked ✓ / – rasterized at 2x the logical mark size and pinned back
// down via `.layout()` — the checkbox widget's mark idiom, white in
// both built-in themes.
fn check_path() -> super::super::vector::PathNode {
    super::super::vector::path(24, 24)
        .move_to(5.0, 12.5)
        .line_to(10.0, 18.0)
        .line_to(19.0, 6.5)
        .stroke(WHITE, 3.0)
        .layout(LayoutStyle::block().fixed_size(MARK, MARK))
}

fn dash_path() -> super::super::vector::PathNode {
    super::super::vector::path(24, 24)
        .move_to(5.0, 12.0)
        .line_to(19.0, 12.0)
        .stroke(WHITE, 3.0)
        .layout(LayoutStyle::block().fixed_size(MARK, MARK))
}

const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
