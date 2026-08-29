use std::cell::RefCell;
use std::rc::Rc;

use sigil_animation_runtime::{AttachedAnimators, install_animators_themed};
use sigil_kernel::{Node, NodeBuilder};
use sigil_reactive::effect_named;
use sigil_style::{
    AnimatorOverlay, ColorAtomKind, ComponentId, NumericAtomKind, Rgba8, StateSlot, StateStyle,
    StyleAtom, StyleBinding, Theme,
};

use crate::interaction::InteractionState;

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

fn install_on_node(
    node: &mut Node,
    base: &'static [StyleAtom],
    state_style: StateStyle,
    component_id: ComponentId,
    interaction: InteractionState,
) {
    let default_theme = Theme::default();
    let theme = node.get_extension::<Theme>().unwrap_or(&default_theme);

    let attached = install_animators_themed(base, state_style, component_id, theme);
    if attached.is_empty() {
        return;
    }

    let Some(binding) = node.get_extension_mut::<StyleBinding>() else {
        return;
    };

    let shared = Rc::new(RefCell::new(attached));
    let overlay = SharedOverlay(Rc::clone(&shared));
    binding.set_animator_overlay(Some(Box::new(overlay)));

    let retarget_rc = Rc::clone(&shared);
    effect_named("sigil::transition_seam::retarget", move || {
        let slot = interaction.style_state().slot();
        retarget_rc.borrow_mut().retarget_to_slot(slot, base);
    });
}

fn has_transition_atom(atoms: &[StyleAtom]) -> bool {
    atoms
        .iter()
        .any(|atom| matches!(atom, StyleAtom::Transition(_)))
}
