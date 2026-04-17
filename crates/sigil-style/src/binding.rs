//! Node style binding storage.

use std::cell::Cell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::{RenderStyle, StyleAtom};

/// Shared reactive style slice updated by a node-owned effect.
pub type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>;

/// Style atoms and resolved caches attached to a node.
#[derive(Debug, Clone)]
pub struct StyleBinding {
    static_atoms: &'static [StyleAtom],
    dynamic_atoms: SmallVec<[StyleAtom; 16]>,
    reactive_atoms: Option<ReactiveStyleAtoms>,
    resolved_render: RenderStyle,
    cx_generation: u64,
    layout_generation: u64,
    theme_generation: u64,
}

impl Default for StyleBinding {
    fn default() -> Self {
        Self {
            static_atoms: &[],
            dynamic_atoms: SmallVec::new(),
            reactive_atoms: None,
            resolved_render: RenderStyle::default(),
            cx_generation: 0,
            layout_generation: 0,
            theme_generation: 0,
        }
    }
}

impl StyleBinding {
    /// Build a binding from static atoms.
    #[must_use]
    pub fn static_atoms(static_atoms: &'static [StyleAtom]) -> Self {
        Self {
            static_atoms,
            ..Self::default()
        }
    }

    /// Build a binding from owned atoms.
    #[must_use]
    pub fn owned_atoms(dynamic_atoms: SmallVec<[StyleAtom; 16]>) -> Self {
        Self {
            dynamic_atoms,
            ..Self::default()
        }
    }

    /// Build a binding from a reactive atom source.
    #[must_use]
    pub fn reactive_atoms(reactive_atoms: ReactiveStyleAtoms) -> Self {
        Self {
            reactive_atoms: Some(reactive_atoms),
            ..Self::default()
        }
    }

    /// Borrow static atoms.
    #[must_use]
    pub const fn static_slice(&self) -> &'static [StyleAtom] {
        self.static_atoms
    }

    /// Borrow dynamic atoms.
    #[must_use]
    pub fn dynamic_slice(&self) -> &[StyleAtom] {
        &self.dynamic_atoms
    }

    /// Borrow the current reactive atom slice.
    #[must_use]
    pub fn reactive_slice(&self) -> &'static [StyleAtom] {
        self.reactive_atoms
            .as_ref()
            .map_or(&[], |atoms| atoms.get())
    }

    /// Borrow cached render style.
    #[must_use]
    pub const fn resolved_render(&self) -> &RenderStyle {
        &self.resolved_render
    }

    /// Return the cached context generation.
    #[must_use]
    pub const fn cx_generation(&self) -> u64 {
        self.cx_generation
    }

    /// Return the cached layout generation.
    #[must_use]
    pub const fn layout_generation(&self) -> u64 {
        self.layout_generation
    }

    /// Return the cached theme generation.
    #[must_use]
    pub const fn theme_generation(&self) -> u64 {
        self.theme_generation
    }

    pub(crate) fn all_atoms(&self) -> SmallVec<[StyleAtom; 32]> {
        let mut atoms = SmallVec::new();
        atoms.extend_from_slice(self.static_atoms);
        atoms.extend_from_slice(&self.dynamic_atoms);
        atoms
    }

    pub(crate) fn replace_owned(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.static_atoms = &[];
        self.dynamic_atoms = atoms;
        self.invalidate();
    }

    pub(crate) fn set_reactive(&mut self, atoms: ReactiveStyleAtoms) {
        self.reactive_atoms = Some(atoms);
        self.invalidate();
    }

    pub(crate) const fn invalidate_theme(&mut self) {
        self.theme_generation = u64::MAX;
    }

    const fn invalidate(&mut self) {
        self.cx_generation = u64::MAX;
        self.layout_generation = u64::MAX;
        self.theme_generation = u64::MAX;
    }
}

impl PartialEq for StyleBinding {
    fn eq(&self, other: &Self) -> bool {
        self.static_atoms == other.static_atoms
            && self.dynamic_atoms == other.dynamic_atoms
            && self.reactive_slice() == other.reactive_slice()
            && self.resolved_render == other.resolved_render
            && self.cx_generation == other.cx_generation
            && self.layout_generation == other.layout_generation
            && self.theme_generation == other.theme_generation
    }
}

impl Eq for StyleBinding {}
