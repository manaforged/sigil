use std::rc::Rc;

use crate::events::{MouseButton, NodeEventsExt};
use crate::kernel::{AccessibleName, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::Tone;

use super::super::checkbox::{CheckMark, checkbox_indicator};
use super::super::interaction::InteractionState;
use super::state::TableState;

pub(super) const WIDTH: f32 = 32.0;

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
    let mark = derived(move || {
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
    let checked = derived(move || mark.get() == Mark::Full);
    let partial = derived(move || mark.get() == Mark::Partial);
    let interaction = InteractionState::new(true);
    let node = container()
        .role(Role::CheckBox)
        .label(label)
        .checked(checked)
        .hittable(true)
        .layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .child(checkbox_indicator(
            CheckMark { checked, partial },
            None,
            Tone::Primary,
            interaction,
        ))
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
        });
    interaction.observe_style(node)
}

fn cell_layout(height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(WIDTH, height)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn header_mark<Row: 'static>(state: TableState<Row>) -> Signal<Mark> {
    derived(move || {
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

impl<Row: 'static> TableState<Row> {
    pub fn select_by(&self, delta: i32) {
        let visible = self.visible().peek();
        let Some(last) = visible.len().checked_sub(1) else {
            return;
        };
        let step = usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX);
        let current = self
            .anchor()
            .and_then(|row| visible.iter().position(|&index| index == row));
        let target = match current {
            Some(at) if delta < 0 => at.saturating_sub(step),
            Some(at) => at.saturating_add(step).min(last),
            None if delta < 0 => last,
            None => 0,
        };
        if let Some(&row) = visible.get(target) {
            self.select_only(row);
        }
    }
}
