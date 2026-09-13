use std::cell::Cell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::style::{AnimatorOverlay, StateSlot, StyleAtom, StyleOverlay};

pub type ReactiveStateSlot = Rc<Cell<StateSlot>>;

pub type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>;

pub struct StyleBinding {
    static_atoms: &'static [StyleAtom],
    dynamic_atoms: SmallVec<[StyleAtom; 16]>,
    reactive_channels: SmallVec<[ReactiveStyleAtoms; 2]>,
    attached_animators: Option<Box<dyn AnimatorOverlay>>,
    state_overlay: Option<Box<StyleOverlay>>,
    state_overlay_slot: Option<ReactiveStateSlot>,
}

impl core::fmt::Debug for StyleBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StyleBinding")
            .field("static_atoms", &self.static_atoms)
            .field("dynamic_atoms", &self.dynamic_atoms)
            .field("reactive_channels", &self.reactive_channels.len())
            .field("attached_animators", &self.attached_animators.is_some())
            .field("state_overlay", &self.state_overlay.is_some())
            .field("state_overlay_slot", &self.state_overlay_slot.is_some())
            .finish()
    }
}

impl Default for StyleBinding {
    fn default() -> Self {
        Self {
            static_atoms: &[],
            dynamic_atoms: SmallVec::new(),
            reactive_channels: SmallVec::new(),
            attached_animators: None,
            state_overlay: None,
            state_overlay_slot: None,
        }
    }
}

impl StyleBinding {
    #[must_use]
    pub fn static_atoms(static_atoms: &'static [StyleAtom]) -> Self {
        Self {
            static_atoms,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn owned_atoms(dynamic_atoms: SmallVec<[StyleAtom; 16]>) -> Self {
        Self {
            dynamic_atoms,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn reactive_atoms(reactive_atoms: ReactiveStyleAtoms) -> Self {
        let mut binding = Self::default();
        binding.add_reactive(reactive_atoms);
        binding
    }

    #[must_use]
    pub const fn static_slice(&self) -> &'static [StyleAtom] {
        self.static_atoms
    }

    #[must_use]
    pub fn dynamic_slice(&self) -> &[StyleAtom] {
        &self.dynamic_atoms
    }

    pub fn reactive_channels(&self) -> impl Iterator<Item = &'static [StyleAtom]> + '_ {
        self.reactive_channels.iter().map(|atoms| atoms.get())
    }

    #[must_use]
    pub fn reactive_contains(&self, target: &StyleAtom) -> bool {
        self.reactive_channels().any(|slice| slice.contains(target))
    }

    #[must_use]
    pub fn has_reactive(&self) -> bool {
        !self.reactive_channels.is_empty()
    }

    #[must_use]
    pub fn hides(&self) -> bool {
        let none = StyleAtom::Display(crate::style::DisplayKind::None);
        self.static_slice().contains(&none)
            || self.dynamic_slice().contains(&none)
            || self.reactive_contains(&none)
    }

    #[must_use]
    pub fn animator_overlay(&self) -> Option<&dyn crate::style::AnimatorOverlay> {
        self.attached_animators.as_deref()
    }

    pub fn set_animator_overlay(
        &mut self,
        overlay: Option<Box<dyn crate::style::AnimatorOverlay>>,
    ) {
        self.attached_animators = overlay;
    }

    pub fn retarget_animators(&mut self, slot: crate::style::StateSlot, base: &[StyleAtom]) {
        if let Some(overlay) = self.attached_animators.as_deref_mut() {
            overlay.retarget_to_slot(slot, base);
        }
    }

    #[must_use]
    pub fn state_overlay(&self) -> Option<&StyleOverlay> {
        self.state_overlay.as_deref()
    }

    #[must_use]
    pub fn interaction_slot(&self) -> StateSlot {
        self.state_overlay_slot
            .as_ref()
            .map_or(StateSlot::Rest, |cell| cell.get())
    }

    #[must_use]
    pub fn state_overlay_atoms(&self) -> &[StyleAtom] {
        let Some(overlay) = self.state_overlay.as_deref() else {
            return &[];
        };
        let slot = self
            .state_overlay_slot
            .as_ref()
            .map_or(StateSlot::Rest, |cell| cell.get());
        overlay.atoms_for(slot)
    }

    pub fn set_state_overlay(&mut self, overlay: StyleOverlay, slot: ReactiveStateSlot) {
        self.state_overlay = Some(Box::new(overlay));
        self.state_overlay_slot = Some(slot);
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
    }

    pub(crate) fn add_reactive(&mut self, atoms: ReactiveStyleAtoms) {
        self.reactive_channels.push(atoms);
    }
}

impl PartialEq for StyleBinding {
    fn eq(&self, other: &Self) -> bool {
        let reactive_eq = self.reactive_channels.len() == other.reactive_channels.len()
            && self
                .reactive_channels()
                .zip(other.reactive_channels())
                .all(|(a, b)| a == b);
        self.static_atoms == other.static_atoms
            && self.dynamic_atoms == other.dynamic_atoms
            && reactive_eq
    }
}

impl Eq for StyleBinding {}
