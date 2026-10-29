use smallvec::SmallVec;

use crate::style::state::PerState;
use crate::style::{Px, StateSlot, StyleAtom, StyleOverlay};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
    states: PerState<SmallVec<[StyleAtom; 4]>>,
}

impl Style {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.atoms
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.atoms.is_empty() && !self.has_state_overlays()
    }

    #[must_use]
    pub(crate) fn has_state_overlays(&self) -> bool {
        self.states.each().iter().any(|atoms| !atoms.is_empty())
    }

    pub(crate) fn set_state(&mut self, slot: StateSlot, atoms: SmallVec<[StyleAtom; 16]>) {
        *self.states.get_mut(slot) = atoms.into_iter().collect();
    }

    #[must_use]
    pub(crate) fn take_state_overlay(&mut self) -> StyleOverlay {
        StyleOverlay::from_states(std::mem::take(&mut self.states))
    }

    pub(crate) fn take_atoms(&mut self) -> SmallVec<[StyleAtom; 16]> {
        std::mem::take(&mut self.atoms)
    }

    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        self.atoms.push(atom);
    }

    #[must_use]
    pub(crate) fn radius(mut self, px: i16) -> Self {
        self.push_atom(StyleAtom::BorderRadius(Px(px)));
        self
    }
}
