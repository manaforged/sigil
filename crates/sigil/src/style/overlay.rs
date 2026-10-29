use smallvec::SmallVec;

use crate::style::state::PerState;
use crate::style::{StateSlot, StyleAtom};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleOverlay {
    slots: PerState<SmallVec<[StyleAtom; 4]>>,
}

impl StyleOverlay {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn from_slots(
        rest: SmallVec<[StyleAtom; 4]>,
        hover: SmallVec<[StyleAtom; 4]>,
        focus: SmallVec<[StyleAtom; 4]>,
        pressed: SmallVec<[StyleAtom; 4]>,
        disabled: SmallVec<[StyleAtom; 4]>,
    ) -> Self {
        Self::from_states(PerState::new(rest, focus, hover, pressed, disabled))
    }

    #[must_use]
    pub(crate) const fn from_states(slots: PerState<SmallVec<[StyleAtom; 4]>>) -> Self {
        Self { slots }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.each().iter().all(|atoms| atoms.is_empty())
    }

    #[must_use]
    pub fn rest(&self) -> &[StyleAtom] {
        self.atoms_for(StateSlot::Rest)
    }

    #[must_use]
    pub fn hover(&self) -> &[StyleAtom] {
        self.atoms_for(StateSlot::Hovered)
    }

    #[must_use]
    pub fn focus(&self) -> &[StyleAtom] {
        self.atoms_for(StateSlot::Focused)
    }

    #[must_use]
    pub fn pressed(&self) -> &[StyleAtom] {
        self.atoms_for(StateSlot::Pressed)
    }

    #[must_use]
    pub fn disabled(&self) -> &[StyleAtom] {
        self.atoms_for(StateSlot::Disabled)
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        for (into, from) in self.slots.each_mut().into_iter().zip(other.slots.each()) {
            *into = crate::style::merge_classes(into, from, 0)
                .into_iter()
                .collect();
        }
    }

    #[must_use]
    pub fn atoms_for(&self, slot: StateSlot) -> &[StyleAtom] {
        self.slots.get(slot)
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
