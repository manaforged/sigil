//! Auto-install seam — wires [`AttachedAnimators`] from the animation
//! runtime into a widget's [`StyleBinding`] when the node's base atoms
//! carry a [`StyleAtom::Transition`] id.
//!
//! Call [`install_transition_animators`] near the end of a widget's
//! `base_node` builder chain (before `interaction.install(node)`).
//! The function is a no-op when no `Transition` atom is present, so it
//! is safe to call unconditionally — zero overhead on the cold path.
//!
//! Reactive retarget: a named effect reads `interaction.style_state()`
//! each time any interaction signal changes, then calls
//! `retarget_to_slot` on the shared animator handle so state transitions
//! animate instead of snapping.

use std::cell::RefCell;
use std::rc::Rc;

use sigil_animation_runtime::{AnimationRegistry, AttachedAnimators, install_animators_themed};
use sigil_kernel::{Node, NodeBuilder};
use sigil_reactive::effect_named;
use sigil_style::{
    AnimatorOverlay, ColorAtomKind, ComponentId, NumericAtomKind, Rgba8, StateSlot, StateStyle,
    StyleAtom, StyleBinding, Theme,
};

use crate::interaction::InteractionState;

// ---- SharedOverlay -------------------------------------------------------

/// Wraps `Rc<RefCell<AttachedAnimators>>` so it can be stored as a
/// `Box<dyn AnimatorOverlay>` in `StyleBinding` while the retarget
/// reactive effect holds a second `Rc` clone.
struct SharedOverlay(Rc<RefCell<AttachedAnimators>>);

impl AnimatorOverlay for SharedOverlay {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32> {
        self.0.borrow().value_for(kind)
    }

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8> {
        self.0.borrow().value_for_color(kind)
    }

    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]) {
        self.0.borrow_mut().retarget_to_slot(slot, base);
    }
}

// ---- public entry point --------------------------------------------------

/// Install transition animators on `builder` when `base` contains a
/// [`StyleAtom::Transition`] atom.
///
/// Returns `builder` unchanged (zero-overhead path) when no
/// `Transition` atom is present or when the registry misses the id.
///
/// # Parameters
///
/// - `builder` — the node builder returned from `base_node`.
/// - `base` — the widget's static base atom slice (same slice passed
///   to `.class(base)`). Must be `'static` so the retarget effect
///   can reference it without lifetime tricks.
/// - `state_style` — the widget's state-slot overlays.
/// - `component_id` — unique namespace for registry-miss diagnostics.
/// - `interaction` — the widget's shared interaction signals; the
///   retarget effect reads these to detect state changes.
#[must_use]
pub fn install_transition_animators(
    builder: NodeBuilder,
    base: &'static [StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
    interaction: InteractionState,
) -> NodeBuilder {
    if !has_transition_atom(base) {
        return builder;
    }
    builder.mutate(move |node| {
        install_on_node(node, base, state_style, component_id, interaction);
    })
}

// ---- install impl --------------------------------------------------------

fn install_on_node(
    node: &mut Node,
    base: &'static [StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
    interaction: InteractionState,
) {
    let registry = AnimationRegistry::default();
    // Borrow the Theme extension for colour resolution; fall back to
    // default when the node has no scoped theme.
    let default_theme = Theme::default();
    let theme = node.get_extension::<Theme>().unwrap_or(&default_theme);

    let attached = install_animators_themed(base, state_style, &registry, component_id, theme);
    if attached.is_empty() {
        return;
    }

    let Some(binding) = node.get_extension_mut::<StyleBinding>() else {
        return;
    };

    let shared = Rc::new(RefCell::new(attached));
    let overlay = SharedOverlay(Rc::clone(&shared));
    binding.set_animator_overlay(Some(Box::new(overlay)));

    // Reactive retarget: re-runs whenever any of the interaction signals
    // that `style_state()` reads change.
    let retarget_rc = Rc::clone(&shared);
    effect_named("sigil::transition_seam::retarget", move || {
        let slot = interaction.style_state().slot();
        retarget_rc.borrow_mut().retarget_to_slot(slot, base);
    });
}

// ---- helpers -------------------------------------------------------------

fn has_transition_atom(atoms: &[StyleAtom]) -> bool {
    atoms
        .iter()
        .any(|atom| matches!(atom, StyleAtom::Transition(_)))
}
