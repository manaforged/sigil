use std::cell::{Cell, OnceCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use smallvec::SmallVec;

static TRANSFORM_GENERATION: AtomicU64 = AtomicU64::new(0);

pub(crate) fn bump_transform_generation() {
    TRANSFORM_GENERATION.fetch_add(1, Ordering::Relaxed);
}

#[must_use]
pub(crate) fn transform_generation() -> u64 {
    TRANSFORM_GENERATION.load(Ordering::Relaxed)
}

use crate::animation::Property;
use crate::animation::runtime::style::StyleAnimator;
use crate::kernel::NodeTag;
use crate::reactive::Signal;
use crate::style::merge::SlotSet;
use crate::style::{
    AnimatorOverlay, Color, ColorAtomKind, NumericAtomKind, StateSlot, StyleAtom, StyleOverlay,
    Theme, Tone, merge_classes,
};

pub(crate) type ReactiveStateSlot = Rc<Cell<StateSlot>>;

#[derive(Clone, Copy, Debug)]
struct Driven {
    property: fn(f32) -> Property,
    value: Signal<f32>,
}

kittest_pub! {
    #[doc(hidden)]
    pub type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>;
}

const FLAG_TRANSFORM: u8 = 1 << 0;
const FLAG_ANIMATE: u8 = 1 << 1;
const FLAG_HIDES: u8 = 1 << 2;
const FLAG_OVERLAY_HIDES: u8 = 1 << 3;

const fn is_animation_atom(atom: StyleAtom) -> bool {
    matches!(atom, StyleAtom::Transition(_) | StyleAtom::Animation(_))
}

fn is_display_none_atom(atom: StyleAtom) -> bool {
    atom == StyleAtom::Display(crate::layout::Display::None)
}

kittest_pub! {
    #[derive(Default)]
    #[doc(hidden)]
    pub struct StyleBinding {
        static_atoms: &'static [StyleAtom],
        dynamic_atoms: SmallVec<[StyleAtom; 16]>,
        reactive_channels: SmallVec<[ReactiveStyleAtoms; 2]>,
        state_channels: SmallVec<[(StateSlot, ReactiveStyleAtoms); 1]>,
        attached_animators: OnceCell<Box<dyn AnimatorOverlay>>,
        glide: OnceCell<Box<crate::animation::runtime::layout_motion::Glide>>,
        state_overlay: Option<Box<StyleOverlay>>,
        animated_values: SmallVec<[Driven; 2]>,
        state_overlay_slots: SmallVec<[ReactiveStateSlot; 1]>,
        widget_channels: Option<usize>,
        user_slots: SlotSet,
        revision: u32,
        flags: u8,
    }
}

impl core::fmt::Debug for StyleBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StyleBinding")
            .field("static_atoms", &self.static_atoms)
            .field("dynamic_atoms", &self.dynamic_atoms)
            .field("reactive_channels", &self.reactive_channels.len())
            .field("state_channels", &self.state_channels.len())
            .field(
                "attached_animators",
                &self.attached_animators.get().is_some(),
            )
            .field("animated_values", &self.animated_values)
            .field("state_overlay", &self.state_overlay.is_some())
            .field("state_overlay_slots", &self.state_overlay_slots.len())
            .field("widget_channels", &self.widget_channels)
            .field("user_slots", &self.user_slots)
            .finish_non_exhaustive()
    }
}

impl StyleBinding {
    #[must_use]
    pub(crate) fn static_atoms(static_atoms: &'static [StyleAtom]) -> Self {
        let mut binding = Self {
            static_atoms,
            ..Self::default()
        };
        binding.recompute_may_transform();
        binding
    }

    kittest_pub! {
        #[must_use]
        #[doc(hidden)]
        pub fn owned_atoms(dynamic_atoms: SmallVec<[StyleAtom; 16]>) -> Self {
            let mut binding = Self {
                dynamic_atoms,
                ..Self::default()
            };
            binding.recompute_may_transform();
            binding
        }
    }

    kittest_pub! {
        #[must_use]
        #[doc(hidden)]
        pub fn reactive_atoms(reactive_atoms: ReactiveStyleAtoms) -> Self {
            let mut binding = Self::default();
            binding.add_reactive(reactive_atoms);
            binding
        }
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub const fn static_slice(&self) -> &'static [StyleAtom] {
        self.static_atoms
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn dynamic_slice(&self) -> &[StyleAtom] {
        &self.dynamic_atoms
    }

    kittest_pub! {
        #[doc(hidden)]
        pub fn reactive_channels(&self) -> impl Iterator<Item = &'static [StyleAtom]> + '_ {
            self.reactive_channels
                .iter()
                .chain(self.state_channels.iter().map(|(_, atoms)| atoms))
                .map(|atoms| atoms.get())
        }
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn reactive_contains(&self, target: &StyleAtom) -> bool {
        self.reactive_channels().any(|slice| slice.contains(target))
    }

    kittest_pub! {
        #[must_use]
        #[doc(hidden)]
        pub fn has_reactive(&self) -> bool {
            !self.reactive_channels.is_empty()
                || !self.state_overlay_slots.is_empty()
                || self.attached_animators.get().is_some()
                || !self.animated_values.is_empty()
                || self.has_transition()
        }
    }

    #[must_use]
    pub(crate) const fn revision(&self) -> u32 {
        self.revision
    }

    pub(crate) fn may_transform(&self) -> bool {
        (self.flags & FLAG_TRANSFORM) != 0
            || self.attached_animators.get().is_some()
            || self
                .reactive_channels
                .iter()
                .any(|atoms| atoms.get().iter().copied().any(StyleAtom::may_transform))
    }

    fn recompute_may_transform(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        let mut flags = 0_u8;
        for &atom in self.static_atoms.iter().chain(&self.dynamic_atoms) {
            if atom.may_transform() {
                flags |= FLAG_TRANSFORM;
            }
            if is_animation_atom(atom) {
                flags |= FLAG_ANIMATE;
            }
            if is_display_none_atom(atom) {
                flags |= FLAG_HIDES;
            }
        }
        if let Some(overlay) = self.state_overlay.as_deref() {
            if overlay.may_transform() {
                flags |= FLAG_TRANSFORM;
            }
            if overlay.any(is_animation_atom) {
                flags |= FLAG_ANIMATE;
            }
            if overlay.any(is_display_none_atom) {
                flags |= FLAG_OVERLAY_HIDES;
            }
        }
        if self.animated_values.iter().any(|entry| {
            (entry.property)(0.0)
                .kind()
                .is_some_and(NumericAtomKind::affects_transform)
        }) {
            flags |= FLAG_TRANSFORM;
        }
        self.flags = flags;
    }

    pub(crate) fn has_transition(&self) -> bool {
        (self.flags & FLAG_ANIMATE) != 0
            || self
                .reactive_channels
                .iter()
                .any(|atoms| atoms.get().iter().copied().any(is_animation_atom))
    }

    #[must_use]
    pub(crate) fn hides(&self) -> bool {
        (self.flags & FLAG_HIDES) != 0
            || ((self.flags & FLAG_OVERLAY_HIDES) != 0
                && self
                    .state_overlay_atoms()
                    .iter()
                    .copied()
                    .any(is_display_none_atom))
            || (!self.reactive_channels.is_empty() && self.live_atoms().any(is_display_none_atom))
    }

    pub(crate) fn cascade(&self) -> impl Iterator<Item = StyleAtom> + '_ {
        self.static_atoms
            .iter()
            .chain(&self.dynamic_atoms)
            .copied()
            .chain(self.live_atoms())
            .chain(self.state_overlay_atoms().iter().copied())
            .chain(self.state_live_atoms())
    }

    fn state_live_atoms(&self) -> impl Iterator<Item = StyleAtom> + '_ {
        let slot = (!self.state_channels.is_empty()).then(|| self.interaction_slot());
        self.state_channels
            .iter()
            .filter(move |(state, _)| Some(*state) == slot)
            .flat_map(|(_, atoms)| atoms.get().iter().copied())
    }

    fn live_atoms(&self) -> impl Iterator<Item = StyleAtom> + '_ {
        let user = self.user_slots;
        let widget_channels = self.widget_channels.unwrap_or(0);
        self.reactive_channels
            .iter()
            .enumerate()
            .flat_map(move |(index, channel)| {
                let masked = index < widget_channels && !user.is_empty();
                channel
                    .get()
                    .iter()
                    .copied()
                    .filter(move |atom| !(masked && SlotSet::atom(*atom).overlaps(user)))
            })
    }

    pub(crate) fn seal(&mut self) {
        if self.widget_channels.is_none() {
            self.widget_channels = Some(self.reactive_channels.len());
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub(crate) const fn note_user(&mut self, slots: SlotSet) {
        if self.widget_channels.is_some() {
            self.user_slots = self.user_slots.union(slots);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub(crate) fn strip(&mut self, slots: SlotSet) {
        if slots.is_empty() || !SlotSet::of(&self.all_atoms()).overlaps(slots) {
            return;
        }
        let kept = merge_classes(&[], &self.all_atoms(), 0)
            .into_iter()
            .filter(|atom| !SlotSet::atom(*atom).overlaps(slots))
            .collect();
        self.replace_owned(kept);
    }

    pub(crate) const fn glide(&self) -> &OnceCell<Box<crate::animation::runtime::layout_motion::Glide>> {
        &self.glide
    }

    #[must_use]
    pub(crate) fn animator_overlay(&self) -> Option<&dyn AnimatorOverlay> {
        if self.animated_values.is_empty() && self.glide.get().is_none() {
            self.attached_animators.get().map(Box::as_ref)
        } else {
            Some(self)
        }
    }

    pub(crate) fn bind_animation(&mut self, property: fn(f32) -> Property, value: Signal<f32>) {
        let driven = Driven { property, value };
        let same = core::mem::discriminant(&property(0.0));
        if let Some(entry) = self
            .animated_values
            .iter_mut()
            .find(|entry| core::mem::discriminant(&(entry.property)(0.0)) == same)
        {
            *entry = driven;
        } else {
            self.animated_values.push(driven);
        }
        self.recompute_may_transform();
    }

    pub(crate) fn update_animations(
        &self,
        theme: &Theme,
        tone: Tone,
        inherited_text: Color,
        tag: Option<NodeTag>,
    ) {
        let atoms: SmallVec<[StyleAtom; 32]> = self.cascade().collect();
        if self.attached_animators.get().is_none()
            && let Some(animator) = StyleAnimator::new(&atoms, tag)
        {
            let _ = self.attached_animators.set(Box::new(animator));
        }
        if let Some(overlay) = self.attached_animators.get() {
            overlay.update(&atoms, theme, tone, inherited_text);
        }
    }

    #[must_use]
    pub(crate) fn state_overlay(&self) -> Option<&StyleOverlay> {
        self.state_overlay.as_deref()
    }

    #[must_use]
    pub(crate) fn interaction_slot(&self) -> StateSlot {
        let mut slots = self.state_overlay_slots.iter().map(|cell| cell.get());
        let first = slots.next().unwrap_or(StateSlot::Rest);
        if slots.any(|slot| slot == StateSlot::Disabled) {
            StateSlot::Disabled
        } else {
            first
        }
    }

    #[must_use]
    pub(crate) fn state_overlay_atoms(&self) -> &[StyleAtom] {
        let Some(overlay) = self.state_overlay.as_deref() else {
            return &[];
        };
        overlay.atoms_for(self.interaction_slot())
    }

    pub(crate) fn set_state_overlay(&mut self, overlay: StyleOverlay, slot: ReactiveStateSlot) {
        let merged = match self.state_overlay.take() {
            Some(mut existing) => {
                existing.merge(&overlay);
                existing
            }
            None => Box::new(overlay),
        };
        self.state_overlay = Some(merged);
        if !self
            .state_overlay_slots
            .iter()
            .any(|known| Rc::ptr_eq(known, &slot))
        {
            self.state_overlay_slots.push(slot);
        }
        self.recompute_may_transform();
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
        self.recompute_may_transform();
    }

    pub(crate) fn add_reactive(&mut self, atoms: ReactiveStyleAtoms) {
        self.reactive_channels.push(atoms);
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn add_state_reactive(&mut self, slot: StateSlot, atoms: ReactiveStyleAtoms) {
        self.state_channels.push((slot, atoms));
        self.revision = self.revision.wrapping_add(1);
    }
}

impl PartialEq for StyleBinding {
    fn eq(&self, other: &Self) -> bool {
        let reactive_eq = self.reactive_channels.len() == other.reactive_channels.len()
            && self.state_channels.len() == other.state_channels.len()
            && self
                .reactive_channels()
                .zip(other.reactive_channels())
                .all(|(a, b)| a == b);
        self.static_atoms == other.static_atoms
            && self.dynamic_atoms == other.dynamic_atoms
            && reactive_eq
            && self.widget_channels == other.widget_channels
            && self.user_slots == other.user_slots
    }
}

impl Eq for StyleBinding {}

impl AnimatorOverlay for StyleBinding {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        self.animated_values
            .iter()
            .rev()
            .find_map(|entry| (entry.property)(entry.value.peek()).numeric(kind))
            .or_else(|| {
                self.attached_animators
                    .get()
                    .and_then(|overlay| overlay.value_for(kind))
            })
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Color> {
        self.attached_animators
            .get()
            .and_then(|overlay| overlay.value_for_color(kind))
    }

    fn layout_offset(&self) -> Option<[f32; 2]> {
        self.glide.get().and_then(|glide| glide.offset())
    }

    fn cache_key(&self) -> u64 {
        use core::hash::{Hash as _, Hasher as _};
        let base = self
            .attached_animators
            .get()
            .map_or(0, |overlay| overlay.cache_key());
        let glide = self.layout_offset();
        if self.animated_values.is_empty() && glide.is_none() {
            return base;
        }
        let mut hasher = ahash::AHasher::default();
        base.hash(&mut hasher);
        glide.map(|offset| offset.map(f32::to_bits)).hash(&mut hasher);
        for entry in &self.animated_values {
            let prop = (entry.property)(entry.value.peek());
            core::mem::discriminant(&prop).hash(&mut hasher);
            entry.value.peek().to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }
}
