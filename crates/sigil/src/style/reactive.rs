use std::cell::Cell;
use std::rc::Rc;

use crate::style::{ReactiveStyleAtoms, StyleAtom};

#[derive(Debug, Clone)]
pub struct StyleReactiveEffect {
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn atoms(&self) -> ReactiveStyleAtoms {
        Rc::clone(&self.atoms)
    }

    pub(crate) fn set_slice(&self, atoms: &'static [StyleAtom]) {
        self.atoms.set(atoms);
    }
}
