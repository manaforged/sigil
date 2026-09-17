use std::rc::Rc;

use crate::events::prelude::{MouseButton, NodeEventsExt};
use crate::kernel::{AccessibleName, NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq};
use crate::style::{NodeStyleExt, Rgba8, StyleAtom};

use super::state::TableState;

pub(super) const WIDTH: f32 = 32.0;
const BOX: f32 = 18.0;
const MARK: f32 = 12.0;

static CHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static UNCHECKED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-inset")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mark {
    Empty,
    Partial,
    Full,
}

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

fn header_mark<Row: 'static>(state: TableState<Row>) -> Signal<Mark> {
    derived_eq(move || {
        let visible = state.visible().get();
        let selection = state.selection().get();
        if selection.is_empty() {
            return Mark::Empty;
        }
        let mut marks = visible.iter().map(|row| selection.contains(row));
        match marks.next() {
            Some(first) if marks.any(|selected| selected != first) => Mark::Partial,
            Some(true) => Mark::Full,
            None | Some(false) => Mark::Empty,
        }
    })
}

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
