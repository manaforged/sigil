//! Node style binding storage.

use std::cell::Cell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::{AnimatorOverlay, RenderStyle, StateSlot, StyleAtom, StyleOverlay};

/// Shared current-state-slot cell.
///
/// A widget's reactive style effect captures a clone and writes the
/// slot it just resolved; the resolver reads through the binding to
/// pick the matching per-instance [`StyleOverlay`] slice.
pub type ReactiveStateSlot = Rc<Cell<StateSlot>>;

/// Shared reactive style slice updated by a node-owned effect.
pub type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>;

/// Style atoms and resolved caches attached to a node.
pub struct StyleBinding {
    static_atoms: &'static [StyleAtom],
    dynamic_atoms: SmallVec<[StyleAtom; 16]>,
    reactive_atoms: Option<ReactiveStyleAtoms>,
    /// W5.1b: attached animator overlay produced by the binding-install
    /// hook in `sigil-animation-runtime`. `None` for bindings without
    /// a `Transition` atom (zero-overhead path); `Some` when one or
    /// more numeric atoms differ between base and a state-slot overlay.
    attached_animators: Option<Box<dyn AnimatorOverlay>>,
    /// DS2 Phase 2: per-instance user state overlay attached via
    /// `.hover()` / `.focus()` / `.pressed()` / `.disabled()` on a
    /// widget builder. The resolver applies these atoms AFTER the
    /// widget's reactive slot atoms, so render-pass last-wins
    /// resolution gives the user override priority per property.
    state_overlay: Option<Box<StyleOverlay>>,
    /// Cell shared with the widget's reactive style effect. Each
    /// reactive evaluation publishes the slot it just resolved; the
    /// resolver reads it back to pick the matching slice from
    /// [`Self::state_overlay`].
    state_overlay_slot: Option<ReactiveStateSlot>,
    resolved_render: RenderStyle,
    cx_generation: u64,
    layout_generation: u64,
    theme_generation: u64,
}

impl core::fmt::Debug for StyleBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StyleBinding")
            .field("static_atoms", &self.static_atoms)
            .field("dynamic_atoms", &self.dynamic_atoms)
            .field("reactive_atoms", &self.reactive_atoms)
            .field("attached_animators", &self.attached_animators.is_some())
            .field("state_overlay", &self.state_overlay.is_some())
            .field("state_overlay_slot", &self.state_overlay_slot.is_some())
            .field("resolved_render", &self.resolved_render)
            .field("cx_generation", &self.cx_generation)
            .field("layout_generation", &self.layout_generation)
            .field("theme_generation", &self.theme_generation)
            .finish()
    }
}

impl Default for StyleBinding {
    fn default() -> Self {
        Self {
            static_atoms: &[],
            dynamic_atoms: SmallVec::new(),
            reactive_atoms: None,
            attached_animators: None,
            state_overlay: None,
            state_overlay_slot: None,
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

    /// True when a reactive atom channel is installed on this binding.
    ///
    /// The reactive channel mutates through an `Rc<Cell>` that bypasses
    /// the owning node's `version()` stamp, so consumers that cache by
    /// version (notably the layout engine's per-node sync fast-path)
    /// need to force a re-sync for reactive nodes even when the
    /// reactive slice is currently empty.
    #[must_use]
    pub const fn has_reactive(&self) -> bool {
        self.reactive_atoms.is_some()
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

    /// Borrow the attached animator overlay, if any.
    ///
    /// W5.1b — the style engine consults this overlay at apply time;
    /// when a numeric atom (opacity / radius / padding / gap / font /
    /// line-height) has a driven value, the engine uses the driven
    /// value instead of the literal atom value.
    #[must_use]
    pub fn animator_overlay(&self) -> Option<&dyn crate::AnimatorOverlay> {
        self.attached_animators.as_deref()
    }

    /// Install a fresh animator overlay, replacing any prior overlay.
    ///
    /// Built by the binding-install hook in `sigil-animation-runtime`.
    /// Passing `None` clears any existing overlay.
    pub fn set_animator_overlay(&mut self, overlay: Option<Box<dyn crate::AnimatorOverlay>>) {
        self.attached_animators = overlay;
    }

    /// Re-aim every attached animator at `slot`'s value, using `base`
    /// as the source of fallback atoms.
    ///
    /// No-op when no animators are attached. Called by the reactive
    /// effect that observes state-slot transitions on this binding.
    pub fn retarget_animators(&mut self, slot: crate::StateSlot, base: &[StyleAtom]) {
        if let Some(overlay) = self.attached_animators.as_deref_mut() {
            overlay.retarget_to_slot(slot, base);
        }
    }

    /// Borrow the per-instance user state overlay, if any.
    #[must_use]
    pub fn state_overlay(&self) -> Option<&StyleOverlay> {
        self.state_overlay.as_deref()
    }

    /// Atoms from the user state overlay for the current slot.
    ///
    /// Returns an empty slice when no overlay is installed, the slot
    /// cell is missing, or the current slot has no user atoms.
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

    /// Install a per-instance user state overlay plus the slot cell
    /// the widget's reactive style effect uses to publish the current
    /// interaction state.
    ///
    /// Passing `overlay.is_empty()` and the matching empty slot cell
    /// is harmless — the resolver skips the overlay pass when the
    /// returned slice is empty.
    pub fn set_state_overlay(&mut self, overlay: StyleOverlay, slot: ReactiveStateSlot) {
        self.state_overlay = Some(Box::new(overlay));
        self.state_overlay_slot = Some(slot);
        self.invalidate();
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
