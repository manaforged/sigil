use std::collections::BTreeSet;

use sigil_events::prelude::{ModifiersState, MouseButton, PointerEvent};
use sigil_reactive::Signal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    None,
    Single,
    Multi,
    Extend,
}

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
    #[must_use]
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            selected: BTreeSet::new(),
            anchor: None,
        }
    }

    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    pub fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.selected.iter().copied()
    }

    pub fn click(&mut self, index: usize) {
        self.apply_click(index, false, false);
    }

    pub fn ctrl_click(&mut self, index: usize) {
        self.apply_click(index, false, true);
    }

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

    pub(super) fn idle(self) -> bool {
        self.state.peek().mode() == SelectionMode::None
    }

    pub(super) fn selected_signal(self, index: usize) -> Option<Signal<bool>> {
        if self.state.peek().contains(index) {
            Some(sigil_reactive::signal(true))
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
#[path = "selection_tests.rs"]
mod tests;
