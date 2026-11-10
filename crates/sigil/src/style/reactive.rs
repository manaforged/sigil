use std::cell::Cell;
use std::rc::Rc;

use crate::style::{ReactiveStyleAtoms, StyleAtom};

/// Holds a reactive atom channel that updates a node's style when its value changes.
#[derive(Debug, Clone)]
pub(crate) struct StyleReactiveEffect {
    atoms: ReactiveStyleAtoms,
}

impl Default for StyleReactiveEffect {
    fn default() -> Self {
        Self {
            atoms: Rc::new(Cell::new(&[])),
        }
    }
}

impl StyleReactiveEffect {
    /// Creates a style reactive effect.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn atoms(&self) -> ReactiveStyleAtoms {
        Rc::clone(&self.atoms)
    }

    pub(crate) fn set_slice(&self, atoms: &'static [StyleAtom]) {
        self.atoms.set(atoms);
    }
}
