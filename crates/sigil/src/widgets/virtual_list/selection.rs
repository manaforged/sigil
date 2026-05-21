use std::collections::BTreeSet;

use events::prelude::{ModifiersState, MouseButton, PointerEvent};
use reactive::Signal;

/// Selection behavior for a virtual list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    /// Row clicks do not affect selection.
    None,
    /// A row click replaces selection with the clicked row.
    Single,
    /// Ctrl/cmd-click toggles rows; plain click replaces selection.
    Multi,
    /// Shift-click extends from the anchor; ctrl/cmd-click toggles.
    Extend,
}

/// Current selected row indices and range-extension anchor.
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

#[derive(Clone, Copy)]
struct ClickModifiers {
    extend: bool,
    toggle: bool,
}

impl SelectionState {
    /// Create an empty selection state for the given mode.
    #[must_use]
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected: BTreeSet::new(),
            anchor: None,
        }
    }

    /// Return this state's selection mode.
    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Return whether `index` is selected.
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    /// Iterate selected row indices in ascending order.
    pub fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected.iter().copied()
    }

    /// Apply a plain row click.
    pub fn click(&mut self, index: usize) {
        self.apply_click(index, false, false);
    }

    /// Apply a row click with ctrl/cmd toggling.
    pub fn ctrl_click(&mut self, index: usize) {
        self.apply_click(index, false, true);
    }

    /// Apply a row click with shift range extension.
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
        self.selected.clear();
        let _ = self.selected.insert(index);
        self.anchor = Some(index);
    }

    fn toggle(&mut self, index: usize) {
        if !self.selected.remove(&index) {
            let _ = self.selected.insert(index);
        }
    }

    fn extend(&mut self, index: usize) {
        let anchor = self.anchor.unwrap_or(index);
        self.selected.clear();
        for selected in anchor.min(index)..=anchor.max(index) {
            let _ = self.selected.insert(selected);
        }
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
        let modifiers = ClickModifiers::from(event.modifiers);
        self.state.update(|state| {
            state.apply_click(index, modifiers.extend, modifiers.toggle);
        });
        event.stop_propagation();
    }

    pub(super) fn selected_signal(self, index: usize) -> Option<Signal<bool>> {
        if self.state.peek().contains(index) {
            Some(reactive::signal(true))
        } else {
            None
        }
    }
}

impl From<ModifiersState> for ClickModifiers {
    fn from(modifiers: ModifiersState) -> Self {
        Self {
            extend: modifiers.shift(),
            toggle: modifiers.ctrl() || modifiers.meta(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, missing_docs)]
mod tests {
    use super::*;

    #[test]
    fn selection_single_replaces_on_click() {
        let mut state = SelectionState::new(SelectionMode::Single);

        state.click(3);
        state.click(5);

        assert_eq!(selected(&state), vec![5]);
    }

    #[test]
    fn selection_multi_toggles_on_ctrl_click() {
        let mut state = SelectionState::new(SelectionMode::Multi);

        state.ctrl_click(3);
        state.ctrl_click(5);
        state.ctrl_click(3);

        assert_eq!(selected(&state), vec![5]);
    }

    #[test]
    fn selection_extend_ranges_on_shift_click() {
        let mut state = SelectionState::new(SelectionMode::Extend);

        state.click(2);
        state.shift_click(5);

        assert_eq!(selected(&state), vec![2, 3, 4, 5]);
    }

    #[test]
    fn selection_none_ignores_clicks() {
        let mut state = SelectionState::new(SelectionMode::None);

        state.click(1);

        assert!(selected(&state).is_empty());
    }

    fn selected(state: &SelectionState) -> Vec<usize> {
        state.selected_indices().collect()
    }
}
