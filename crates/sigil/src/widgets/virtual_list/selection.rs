use std::collections::BTreeSet;

use crate::events::{MouseButton, PointerEvent};
use crate::reactive::Signal;
use crate::widgets::listbox::{only, span, toggle};

/// Selection mode of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SelectionMode {
    /// Rows cannot be selected.
    None,
    /// One row is selected at a time.
    Single,
    /// A click selects one row; Ctrl-click or Cmd-click adds or removes a row.
    Multi,
    /// The same as `Multi`, and Shift-click selects the range from the anchor row.
    Extend,
}

/// Selected indexes of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    mode: SelectionMode,
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) struct SelectionRuntime {
    state: Signal<Selection>,
}

impl Selection {
    /// Creates an empty selection with `mode`.
    #[must_use]
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected: BTreeSet::new(),
            anchor: None,
        }
    }

    /// Returns the mode that decides how many rows a click can select.
    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    pub(super) const fn set_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
    }

    /// Returns whether `index` is selected.
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    /// The selected indexes in ascending order.
    pub fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected.iter().copied()
    }

    /// Removes every index.
    pub fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    /// Selects only `index`.
    pub fn select(&mut self, index: usize) {
        self.apply_click(index, false, false);
    }

    /// Flips the selection of `index`; in `Single` mode it selects only `index`.
    pub fn toggle(&mut self, index: usize) {
        self.apply_click(index, false, true);
    }

    /// Selects the range from the anchor to `index`; outside `Extend` mode it selects only `index`.
    pub fn extend(&mut self, index: usize) {
        self.apply_click(index, true, false);
    }

    fn apply_click(&mut self, index: usize, extend: bool, toggle: bool) {
        match self.mode {
            SelectionMode::None => {}
            SelectionMode::Extend if extend => self.span_to(index),
            SelectionMode::Multi | SelectionMode::Extend if toggle => self.flip(index),
            SelectionMode::Single | SelectionMode::Multi | SelectionMode::Extend => {
                self.replace(index);
            }
        }
    }

    fn replace(&mut self, index: usize) {
        only(&mut self.selected, &mut self.anchor, index);
    }

    fn flip(&mut self, index: usize) {
        toggle(&mut self.selected, index);
        self.anchor = Some(index);
    }

    fn span_to(&mut self, index: usize) {
        let anchor = self.anchor.unwrap_or(index);
        span(&mut self.selected, anchor.min(index)..=anchor.max(index));
        self.anchor = Some(anchor);
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::new(SelectionMode::Single)
    }
}

impl From<SelectionMode> for Selection {
    fn from(mode: SelectionMode) -> Self {
        Self::new(mode)
    }
}

impl SelectionRuntime {
    pub(super) const fn new(state: Signal<Selection>) -> Self {
        Self { state }
    }

    pub(super) fn apply_pointer_down(self, index: usize, event: &mut PointerEvent) {
        if event
            .button
            .is_some_and(|button| button != MouseButton::Left)
        {
            return;
        }
        let modifiers = event.modifiers;
        let toggle = modifiers.ctrl() || modifiers.meta();
        self.state
            .update(|state| state.apply_click(index, modifiers.shift(), toggle));
        event.stop_propagation();
    }

    pub(super) fn idle(self) -> bool {
        self.state.peek().mode() == SelectionMode::None
    }

    pub(super) fn selected_signal(self, index: usize) -> Option<Signal<bool>> {
        self.state
            .peek()
            .contains(index)
            .then(|| crate::reactive::signal(true))
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
