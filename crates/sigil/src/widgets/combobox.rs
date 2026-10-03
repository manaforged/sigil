use crate::events::NodeEventsExt;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::PopupSide;
use crate::layout::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::Reactive;
use crate::reactive::{Signal, derived, signal};
use crate::style::{ControlSize, Metrics, space};

use super::layout_ext::WidgetLayoutExt;
use super::listbox::{ListModel, listbox, scrolled};
use super::popup::{self, ItemSlots, POPUP_GAP, item_row};
use super::text_field::{TextFieldBuilder, text_field};

const MAX_VISIBLE: usize = 8;
const FIELD_HEIGHT: f32 = space(8);

#[derive(Clone, Copy)]
pub struct Combobox {
    options: Signal<Vec<String>>,
    query: Signal<String>,
    selected: Signal<Option<usize>>,
    open: Signal<bool>,
    cursor: Signal<usize>,
    session: Signal<u32>,
    placeholder: &'static str,
    disabled: Option<Signal<bool>>,
}

impl Combobox {
    #[must_use]
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options: signal(options),
            query: signal(String::new()),
            selected: signal(None),
            open: signal(false),
            cursor: signal(0),
            session: signal(0),
            placeholder: "",
            disabled: None,
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub const fn placeholder(mut self, text: &'static str) -> Self {
        self.placeholder = text;
        self
    }

    pub fn query(self) -> String {
        self.query.get()
    }

    pub fn selected(self) -> Option<usize> {
        self.selected.get()
    }

    pub fn selected_option(self) -> Option<String> {
        self.selected
            .get()
            .and_then(|index| self.options.peek().get(index).cloned())
    }

    pub const fn selected_signal(self) -> Signal<Option<usize>> {
        self.selected
    }

    pub fn is_open(self) -> bool {
        self.open.get()
    }

    pub fn open(self) {
        let row = self
            .selected
            .peek()
            .and_then(|index| self.filtered().iter().position(|(i, _)| *i == index))
            .unwrap_or(0);
        self.cursor.set(row);
        self.open.set(true);
    }

    pub fn close(self) {
        self.open.set(false);
    }

    pub fn filtered(self) -> Vec<(usize, String)> {
        let query = self.query.get().to_lowercase();
        self.options
            .get()
            .iter()
            .enumerate()
            .filter(|(_, label)| query.is_empty() || label.to_lowercase().contains(&query))
            .map(|(index, label)| (index, label.clone()))
            .collect()
    }

    pub fn cursor(self) -> usize {
        self.cursor.get()
    }

    pub fn edit_query(self, text: &str) {
        self.query.update(|q| {
            q.clear();
            q.push_str(text);
        });
        self.edit(text);
    }

    pub fn move_cursor(self, delta: i32) {
        let len = self.filtered().len();
        if len == 0 {
            self.cursor.set(0);
            return;
        }
        let current = self.cursor.peek();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            current
                .saturating_add(usize::try_from(delta).unwrap_or_default())
                .min(len - 1)
        };
        self.cursor.set(next);
    }

    pub fn commit(self, index: usize) {
        let Some(label) = self.options.peek().get(index).cloned() else {
            return;
        };
        self.selected.set(Some(index));
        self.query.set(label);
        self.open.set(false);
        self.session.update(|s| *s += 1);
    }

    pub fn commit_cursor(self) -> bool {
        let Some(&(index, _)) = self.filtered().get(self.cursor.peek()) else {
            return false;
        };
        self.commit(index);
        true
    }

    fn edit(self, text: &str) {
        self.open.set(true);
        self.cursor.set(0);
        let matches_selection = self
            .selected
            .peek()
            .and_then(|index| self.options.peek().get(index).cloned())
            .is_some_and(|label| label == text);
        if !matches_selection {
            self.selected.set(None);
        }
    }
}

impl ListModel for Combobox {
    fn cursor(&self) -> Signal<usize> {
        self.cursor
    }

    fn len(&self) -> usize {
        self.filtered().len()
    }

    fn is_open(&self) -> bool {
        self.open.peek()
    }

    fn open(&self) {
        Self::open(*self);
    }

    fn commit(&self, index: usize) {
        if let Some(&(option, _)) = self.filtered().get(index) {
            Self::commit(*self, option);
        }
    }

    fn close(&self) {
        Self::close(*self);
    }

    fn text_input(&self) -> bool {
        true
    }
}

#[must_use]
pub fn combobox(state: Combobox) -> NodeBuilder {
    let label = if state.placeholder.is_empty() {
        "Combobox"
    } else {
        state.placeholder
    };
    listbox(
        container()
            .role(Role::ComboBox)
            .label(label)
            .expanded(state.open)
            .layout(LayoutStyle::block().fill_width().height(FIELD_HEIGHT)),
        state,
    )
    .child(field_host(state))
    .child(popup_host(state))
}

fn field_host(state: Combobox) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width())
        .pointer_down_capture(move |_| {
            if !state.disabled.is_some_and(|disabled| disabled.peek()) && !state.open.peek() {
                state.open();
            }
        })
        .children_reactive(move || {
            let _ = state.session.get();
            vec![build_field(state)]
        })
}

fn build_field(state: Combobox) -> TextFieldBuilder {
    let field = text_field(state.query);
    let field = match state.disabled {
        Some(disabled) => field.disabled(disabled),
        None => field,
    };
    field
        .placeholder(state.placeholder)
        .blur(move || ListModel::blur(&state))
        .height(FIELD_HEIGHT)
        .change(move |text| state.edit(text))
        .submit(move |_| {
            if state.open.peek() {
                state.commit_cursor();
            }
        })
}

fn popup_host(state: Combobox) -> NodeBuilder {
    container().children_reactive(move || {
        let matches = state.filtered();
        if state.open.get() && !matches.is_empty() {
            vec![build_popup(state, matches)]
        } else {
            Vec::new()
        }
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "row counts are small; sub-unit precision is irrelevant at render scale"
)]
fn rows_height(count: usize) -> f32 {
    count as f32 * Metrics::current().control_height(ControlSize::Default)
}

fn build_popup(state: Combobox, matches: Vec<(usize, String)>) -> NodeBuilder {
    let count = matches.len();
    let extent = rows_height(count);
    let view_h = rows_height(count.min(MAX_VISIBLE));
    let frame = popup::popup(Role::ListBox, PopupSide::Below).layout(
        LayoutStyle::flex_column()
            .fill_width()
            .height(2.0f32.mul_add(popup::inset(), view_h))
            .padding(Metrics::current().overlay_padding())
            .position(Position::Absolute)
            .inset_top(FIELD_HEIGHT + POPUP_GAP)
            .inset_left(0.0),
    );
    let mut list = container().layout(LayoutStyle::flex_column().fill_width().height(extent));
    for (row, (index, label)) in matches.into_iter().enumerate() {
        list = list.child(build_row(state, row, index, label));
    }
    if count > MAX_VISIBLE {
        frame.child(scrolled(
            list,
            state.cursor,
            rows_height(1),
            extent,
            (None, view_h),
        ))
    } else {
        frame.child(list)
    }
}

fn build_row(state: Combobox, row: usize, index: usize, label: String) -> NodeBuilder {
    let checked = derived(move || state.selected.get() == Some(index));
    let highlight = derived(move || state.cursor.get() == row);
    let slots = ItemSlots {
        checked: Some(checked),
        ..ItemSlots::new(label)
    };
    item_row(slots, highlight)
        .role(Role::ListBoxOption)
        .selected(checked)
        .pointer_up(move |ev| {
            if ev.is_captured() {
                return;
            }
            state.commit(index);
            ev.stop_propagation();
        })
        .pointer_enter(move |_| {
            state.cursor.set(row);
        })
}

#[cfg(test)]
#[path = "combobox_tests.rs"]
mod tests;
