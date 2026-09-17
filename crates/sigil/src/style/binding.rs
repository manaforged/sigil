use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use smallvec::SmallVec;

use crate::animation_runtime::style::StyleAnimator;
use crate::reactive::Signal;
use crate::style::{
    AnimatorOverlay, ColorAtomKind, NumericAtomKind, Rgba8, StateSlot, StyleAtom, StyleOverlay,
    Theme, Tone,
};

pub type ReactiveStateSlot = Rc<Cell<StateSlot>>;

pub type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>;

pub struct StyleBinding {
    static_atoms: &'static [StyleAtom],
    dynamic_atoms: SmallVec<[StyleAtom; 16]>,
    reactive_channels: SmallVec<[ReactiveStyleAtoms; 2]>,
    attached_animators: OnceCell<Box<dyn AnimatorOverlay>>,
    state_overlay: Option<Box<StyleOverlay>>,
    animated_values: SmallVec<[(NumericAtomKind, Signal<f32>); 2]>,
    state_overlay_slot: Option<ReactiveStateSlot>,
}

impl core::fmt::Debug for StyleBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StyleBinding")
            .field("static_atoms", &self.static_atoms)
            .field("dynamic_atoms", &self.dynamic_atoms)
            .field("reactive_channels", &self.reactive_channels.len())
            .field(
                "attached_animators",
                &self.attached_animators.get().is_some(),
            )
            .field("animated_values", &self.animated_values)
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
            attached_animators: OnceCell::new(),
            state_overlay: None,
            animated_values: SmallVec::new(),
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
            || self.state_overlay_slot.is_some()
            || self.attached_animators.get().is_some()
            || !self.animated_values.is_empty()
            || self.has_transition()
    }

    pub(crate) fn may_transform(&self) -> bool {
        self.has_reactive()
            || self
                .static_atoms
                .iter()
                .chain(&self.dynamic_atoms)
                .any(|atom| matches!(atom, StyleAtom::Transform(_)))
    }

    pub(crate) fn has_transition(&self) -> bool {
        self.static_atoms
            .iter()
            .chain(&self.dynamic_atoms)
            .chain(self.state_overlay_atoms())
            .copied()
            .chain(self.reactive_channels().flatten().copied())
            .any(|atom| matches!(atom, StyleAtom::Transition(_)))
    }

    #[must_use]
    pub fn hides(&self) -> bool {
        let none = StyleAtom::Display(crate::style::DisplayKind::None);
        self.static_slice().contains(&none)
            || self.dynamic_slice().contains(&none)
            || self.reactive_contains(&none)
    }

    #[must_use]
    pub fn animator_overlay(&self) -> Option<&dyn AnimatorOverlay> {
        if self.animated_values.is_empty() {
            self.attached_animators.get().map(Box::as_ref)
        } else {
            Some(self)
        }
    }

    pub(crate) fn bind_animation(&mut self, kind: NumericAtomKind, value: Signal<f32>) {
        if let Some(entry) = self
            .animated_values
            .iter_mut()
            .find(|entry| entry.0 == kind)
        {
            entry.1 = value;
        } else {
            self.animated_values.push((kind, value));
        }
    }

    pub fn set_animator_overlay(&mut self, overlay: Option<Box<dyn AnimatorOverlay>>) {
        self.attached_animators = overlay.map_or_else(OnceCell::new, OnceCell::from);
    }

    pub fn retarget_animators(&mut self, slot: StateSlot, base: &[StyleAtom]) {
        if let Some(overlay) = self.attached_animators.get_mut() {
            overlay.retarget_to_slot(slot, base);
        }
    }

    pub(crate) fn update_animations(&self, theme: &Theme, tone: Tone, inherited_text: Rgba8) {
        let mut atoms = self.all_atoms();
        for channel in self.reactive_channels() {
            atoms.extend_from_slice(channel);
        }
        atoms.extend_from_slice(self.state_overlay_atoms());
        if self.attached_animators.get().is_none()
            && let Some(animator) = StyleAnimator::new(&atoms)
        {
            let _ = self.attached_animators.set(Box::new(animator));
        }
        if let Some(overlay) = self.attached_animators.get() {
            overlay.update(&atoms, theme, tone, inherited_text);
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

impl AnimatorOverlay for StyleBinding {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        self.animated_values
            .iter()
            .find(|entry| entry.0 == kind)
            .map(|entry| entry.1.peek())
            .or_else(|| {
                self.attached_animators
                    .get()
                    .and_then(|overlay| overlay.value_for(kind))
            })
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        self.attached_animators
            .get()
            .and_then(|overlay| overlay.value_for_color(kind))
    }

    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]) {
        self.retarget_animators(slot, base);
    }
}
