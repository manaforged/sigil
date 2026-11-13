use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::PopupSide;
use crate::layout::Space;
use crate::layout::{LayoutStyle, Position};
use crate::reactive::Reactive;
use crate::reactive::{Signal, batch, memo, signal};
use crate::style::{ControlSize, Metrics, space};
use crate::widgets::{Layout, Widget};

use super::listbox::{ListModel, Matcher, listbox, scrolled};
use super::popup::{self, ItemSlots, POPUP_GAP, item_row};
use super::shared::disabled_now;
use super::text_field::{TextFieldBuilder, text_field};

const MAX_VISIBLE: usize = 8;
const FIELD_HEIGHT: f32 = space(8);

/// State of a combobox.
#[derive(Clone, Copy, Debug)]
pub struct Combobox {
    options: Signal<Vec<String>>,
    query: Signal<String>,
    selected: Signal<Option<usize>>,
    open: Signal<bool>,
    cursor: Signal<usize>,
    placeholder: Signal<String>,
    disabled: Option<Signal<bool>>,
}

impl Combobox {
    /// Creates the state of a combobox with `options`, an empty query, and no selection.
    #[must_use]
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options: signal(options),
            query: signal(String::new()),
            selected: signal(None),
            open: signal(false),
            cursor: signal(0),
            placeholder: signal(String::new()),
            disabled: None,
        }
    }

    /// Stops the list from opening and options from being chosen while `disabled` is true.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the placeholder text of the input field.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = signal(text.into());
        self
    }

    pub(crate) const fn placeholder_text(self) -> Signal<String> {
        self.placeholder
    }

    pub(crate) const fn disabled_flag(self) -> Option<Signal<bool>> {
        self.disabled
    }

    /// The text in the input field; a read in a reactive closure subscribes to it.
    #[must_use]
    pub fn query(self) -> String {
        self.query.get()
    }

    /// The index of the chosen option in the full list; `None` when nothing is chosen.
    #[must_use]
    pub fn selected(self) -> Option<usize> {
        self.selected.get()
    }

    /// The label of the chosen option; `None` when nothing is chosen.
    #[must_use]
    pub fn selected_option(self) -> Option<String> {
        self.selected
            .get()
            .and_then(|index| self.options.peek().get(index).cloned())
    }

    /// The signal that holds the index of the chosen option.
    #[must_use]
    pub const fn selected_signal(self) -> Signal<Option<usize>> {
        self.selected
    }

    /// Returns whether this combobox is open.
    #[must_use]
    pub fn is_open(self) -> bool {
        self.open.get()
    }

    /// Opens the list.
    pub fn open(self) {
        let row = self
            .selected
            .peek()
            .and_then(|index| self.filtered().iter().position(|(i, _)| *i == index))
            .unwrap_or(0);
        self.reveal(row);
    }

    /// Closes the list.
    pub fn close(self) {
        self.open.set(false);
    }

    /// The options that match the query, best match first, each with its index in the full list.
    pub fn filtered(self) -> Vec<(usize, String)> {
        let query = self.query.get();
        let options = self.options.get();
        Matcher::Substring
            .rank(&query, options.iter().map(String::as_str))
            .into_iter()
            .filter_map(|index| options.get(index).map(|label| (index, label.clone())))
            .collect()
    }

    /// The row of the highlighted option in the filtered list.
    #[must_use]
    pub fn cursor(self) -> usize {
        self.cursor.get()
    }

    /// Replaces the query text.
    pub fn edit_query(self, text: &str) {
        self.query.update(|q| {
            q.clear();
            q.push_str(text);
        });
        self.edit(text);
    }

    /// Moves the list cursor by `delta`.
    pub fn move_cursor(self, delta: i32) {
        let len = self.filtered().len();
        if len == 0 {
            self.cursor.set(0);
            return;
        }
        let next = self
            .cursor
            .peek()
            .saturating_add_signed(isize::try_from(delta).unwrap_or_default());
        self.cursor
            .set(if delta < 0 { next } else { next.min(len - 1) });
    }

    /// Chooses the option at `index`.
    pub fn commit(self, index: usize) {
        if disabled_now(self.disabled) {
            return;
        }
        let Some(label) = self.options.peek().get(index).cloned() else {
            return;
        };
        batch(|| {
            self.selected.set(Some(index));
            self.query.set(label);
            self.open.set(false);
        });
    }

    /// Commits the highlighted row and returns whether a row was committed.
    #[allow(
        clippy::must_use_candidate,
        reason = "commits the highlighted row; the result is optional"
    )]
    pub fn commit_cursor(self) -> bool {
        let Some(&(index, _)) = self.filtered().get(self.cursor.peek()) else {
            return false;
        };
        self.commit(index);
        true
    }

    fn reveal(self, row: usize) {
        if disabled_now(self.disabled) {
            return;
        }
        self.cursor.set(row);
        self.open.set(true);
    }

    fn shown(self) -> bool {
        self.open.get() && !self.filtered().is_empty()
    }

    fn edit(self, text: &str) {
        self.reveal(0);
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

/// A text input with a filtered list.
#[must_use]
pub fn combobox(state: Combobox) -> NodeBuilder {
    let placeholder = state.placeholder.peek();
    let label = if placeholder.is_empty() {
        String::from("Combobox")
    } else {
        placeholder
    };
    listbox(
        container()
            .role(Role::ComboBox)
            .label(label)
            .expanded(memo(move || state.shown()))
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
            if !state.open.peek() {
                state.open();
            }
        })
        .child(build_field(state))
}

fn build_field(state: Combobox) -> TextFieldBuilder {
    let field = text_field(state.query);
    let field = match state.disabled {
        Some(disabled) => field.disabled(disabled),
        None => field,
    };
    field
        .placeholder(state.placeholder)
        .focus_out(move || ListModel::blur(&state))
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
        if state.shown() {
            vec![build_popup(state, state.filtered())]
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

pub(super) fn list_popup(
    (top, width): (f32, Option<f32>),
    (count, max_visible): (usize, usize),
    cursor: Signal<usize>,
    rows: impl Iterator<Item = NodeBuilder>,
) -> NodeBuilder {
    let view_h = rows_height(count.min(max_visible));
    let extent = rows_height(count);
    let inner_w = width.map(|width| 2.0f32.mul_add(-popup::inset(), width).max(0.0));
    let base = LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(top)
        .inset_left(0.0)
        .height(2.0f32.mul_add(popup::inset(), view_h))
        .padding(Space::px(Metrics::current().overlay_padding()));
    let frame = popup::popup(Role::ListBox, PopupSide::Below).layout(match width {
        Some(width) => base.width(width),
        None => base.fill_width(),
    });
    let list = rows.fold(
        container().layout(LayoutStyle::flex_column().fill_width().height(extent)),
        NodeBuilder::child,
    );
    if count > max_visible {
        frame.child(scrolled(
            list,
            cursor,
            rows_height(1),
            extent,
            (inner_w, view_h),
        ))
    } else {
        frame.child(list)
    }
}

fn build_popup(state: Combobox, matches: Vec<(usize, String)>) -> NodeBuilder {
    list_popup(
        (FIELD_HEIGHT + POPUP_GAP, None),
        (matches.len(), MAX_VISIBLE),
        state.cursor,
        matches
            .into_iter()
            .enumerate()
            .map(move |(row, (index, label))| build_row(state, row, index, label)),
    )
}

fn build_row(state: Combobox, row: usize, index: usize, label: String) -> NodeBuilder {
    let checked = memo(move || state.selected.get() == Some(index));
    let highlight = memo(move || state.cursor.get() == row);
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
