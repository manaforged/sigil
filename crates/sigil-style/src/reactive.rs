//! Reactive style binding support.

use std::cell::Cell;
use std::rc::Rc;

use crate::{ReactiveStyleAtoms, StyleAtom};

/// Node-owned reactive style state.
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
    /// Create an empty reactive style state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the current reactive atom slice.
    #[must_use]
    pub fn current_slice(&self) -> &'static [StyleAtom] {
        self.atoms.get()
    }

    pub(crate) fn atoms(&self) -> ReactiveStyleAtoms {
        Rc::clone(&self.atoms)
    }

    pub(crate) fn set_slice(&self, atoms: &'static [StyleAtom]) {
        self.atoms.set(atoms);
    }
}
