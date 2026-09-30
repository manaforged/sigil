use smallvec::SmallVec;

use crate::style::{Px, StyleAtom};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
    rest_atoms: SmallVec<[StyleAtom; 4]>,
    hover_atoms: SmallVec<[StyleAtom; 4]>,
    focus_atoms: SmallVec<[StyleAtom; 4]>,
    pressed_atoms: SmallVec<[StyleAtom; 4]>,
    disabled_atoms: SmallVec<[StyleAtom; 4]>,
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
        !self.rest_atoms.is_empty()
            || !self.hover_atoms.is_empty()
            || !self.focus_atoms.is_empty()
            || !self.pressed_atoms.is_empty()
            || !self.disabled_atoms.is_empty()
    }

    pub(crate) fn set_rest(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.rest_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_hover(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.hover_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_focus(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.focus_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_pressed(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.pressed_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_disabled(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.disabled_atoms = atoms.into_iter().collect();
    }

    #[must_use]
    pub(crate) fn take_state_overlay(&mut self) -> crate::style::StyleOverlay {
        crate::style::StyleOverlay::from_slots(
            std::mem::take(&mut self.rest_atoms),
            std::mem::take(&mut self.hover_atoms),
            std::mem::take(&mut self.focus_atoms),
            std::mem::take(&mut self.pressed_atoms),
            std::mem::take(&mut self.disabled_atoms),
        )
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
