use smallvec::SmallVec;

use crate::style::state::PerState;
use crate::style::{StateSlot, StyleAtom};

/// Holds extra style atoms for each interaction state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StyleOverlay {
    slots: PerState<SmallVec<[StyleAtom; 4]>>,
}

impl StyleOverlay {
    #[cfg(test)]
    /// Creates a style overlay.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    /// Creates a style overlay from slots.
    #[must_use]
    pub(crate) const fn from_slots(
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

    /// Returns whether this style overlay is empty.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.each().iter().all(|atoms| atoms.is_empty())
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        for (into, from) in self.slots.each_mut().into_iter().zip(other.slots.each()) {
            *into = crate::style::merge_classes(into, from, 0)
                .into_iter()
                .collect();
        }
    }

    /// Returns the atoms of one state slot.
    #[must_use]
    pub(crate) fn atoms_for(&self, slot: StateSlot) -> &[StyleAtom] {
        self.slots.get(slot)
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
