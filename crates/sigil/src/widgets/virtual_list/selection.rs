use std::collections::BTreeSet;

use crate::events::{MouseButton, PointerEvent};
use crate::reactive::Signal;
use crate::widgets::listbox::{only, span, toggle};

/// Selection mode of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    /// The none selection mode.
    None,
    /// The single selection mode.
    Single,
    /// The multi selection mode.
    Multi,
    /// The extend selection mode.
    Extend,
}

/// Selected indexes of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionState {
    mode: SelectionMode,
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) struct SelectionRuntime {
    state: Signal<SelectionState>,
}

impl SelectionState {
    /// Creates a selection state.
    #[must_use]
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected: BTreeSet::new(),
            anchor: None,
        }
    }

    /// Returns the mode.
    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Returns whether `index` is selected.
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    /// Returns the selected indices.
    pub fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected.iter().copied()
    }

    /// Selects only `index`.
    pub fn click(&mut self, index: usize) {
        self.apply_click(index, false, false);
    }

    /// Flips the selection of `index`.
    pub fn ctrl_click(&mut self, index: usize) {
        self.apply_click(index, false, true);
    }

    /// Selects the range from the anchor to `index`.
    pub fn shift_click(&mut self, index: usize) {
        self.apply_click(index, true, false);
    }

    fn apply_click(&mut self, index: usize, extend: bool, toggle: bool) {
        match self.mode {
            SelectionMode::None => {}
            SelectionMode::Extend if extend => self.extend(index),
            SelectionMode::Multi | SelectionMode::Extend if toggle => self.toggle(index),
            SelectionMode::Single | SelectionMode::Multi | SelectionMode::Extend => {
                self.replace(index);
            }
        }
    }

    fn replace(&mut self, index: usize) {
        only(&mut self.selected, &mut self.anchor, index);
    }

    fn toggle(&mut self, index: usize) {
        toggle(&mut self.selected, index);
    }

    fn extend(&mut self, index: usize) {
        let anchor = self.anchor.unwrap_or(index);
        span(&mut self.selected, anchor.min(index)..=anchor.max(index));
        self.anchor = Some(anchor);
    }
}

impl SelectionRuntime {
    pub(super) const fn new(state: Signal<SelectionState>) -> Self {
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
