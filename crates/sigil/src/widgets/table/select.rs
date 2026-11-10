use crate::style::strings::StringKey;
use crate::style::theme::ambient::{ActiveTheme, theme_label, theme_string};
use std::rc::Rc;

use crate::events::MouseButton;
use crate::kernel::{AccessibleName, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::Tone;
use crate::widgets::{Layout, Widget};

use super::super::checkbox::{CheckMark, checkbox_indicator};
use super::super::interaction::InteractionState;
use super::super::listbox::{SelectMark, mark, toggle_all};
use super::state::TableState;

pub(super) const WIDTH: f32 = 32.0;

pub(super) fn header_cell<Row: 'static>(state: &TableState<Row>, height: f32) -> NodeBuilder {
    let toggle = state.clone();
    let check = select_box(
        header_mark(state.clone()),
        theme_string(StringKey::SelectAllRows),
        move || toggle.toggle_all(),
    );
    select_cell(Role::ColumnHeader, height, check).label(theme_label(StringKey::Select))
}

pub(super) fn row_cell<Row: 'static>(
    state: &TableState<Row>,
    row: u32,
    height: f32,
) -> NodeBuilder {
    let selection = state.selection();
    let mark = derived(move || {
        if selection.get().contains(&row) {
            SelectMark::Full
        } else {
            SelectMark::Empty
        }
    });
    let toggle = state.clone();
    let check = select_box(
        mark,
        ActiveTheme::capture().fill(StringKey::SelectRow, &[&(row + 1)]),
        move || toggle.toggle_selected(row),
    );
    select_cell(Role::GridCell, height, check)
}

fn select_cell(role: Role, height: f32, check: NodeBuilder) -> NodeBuilder {
    container()
        .role(role)
        .column_index(1)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(WIDTH, height)
                .shrink(0.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .child(check)
}

fn select_box(
    mark: Signal<SelectMark>,
    label: impl Into<AccessibleName>,
    toggle: impl Fn() + 'static,
) -> NodeBuilder {
    let checked = derived(move || mark.get() == SelectMark::Full);
    let partial = derived(move || mark.get() == SelectMark::Partial);
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

fn header_mark<Row: 'static>(state: TableState<Row>) -> Signal<SelectMark> {
    derived(move || {
        let visible = state.visible().get();
        let selection = state.selection().get();
        mark(&*selection, visible.as_slice())
    })
}

impl<Row: 'static> TableState<Row> {
    fn toggle_all(&self) {
        let visible = self.visible().peek();
        let mut set = (*self.selection().peek()).clone();
        toggle_all(&mut set, visible.as_slice());
        self.selection().set(Rc::new(set));
    }

    /// Moves the selection by `delta` rows.
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
