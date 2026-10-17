use crate::kernel::{DeltaKind, Node};
use crate::reactive::{Runtime, Signal, effect_named};

use crate::style::merge::SlotSet;
use crate::style::{
    ClassSource, FocusRing, NumericAtomKind, ReactiveStateSlot, StyleAtom, StyleBinding,
    StyleOverlay, StyleReactiveEffect, Theme, Tone, ToneContext, merge_classes, parse_classes,
};

pub(super) fn install_animation(node: &mut Node, kind: NumericAtomKind, value: Signal<f32>) {
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.bind_animation(kind, value);
    } else {
        let mut binding = StyleBinding::default();
        binding.bind_animation(kind, value);
        let _previous = node.attach(binding);
    }
    let tag = node.tag();
    let delta = DeltaKind::layout_if(kind.affects_layout());
    effect_named("sigil-style::animation", move || {
        assert!(
            value.get().is_finite(),
            "animated style values must be finite"
        );
        if let Some(tag) = tag {
            tag.record(delta);
        }
        Runtime::mark_frame_dirty();
    });
}

pub(super) fn install_tone(node: &mut Node, tone: Tone) {
    debug_assert_not_fragment(node, "tone");
    let _previous = node.attach(ToneContext::new(tone));
}

pub(super) fn install_state_overlay(
    node: &mut Node,
    overlay: StyleOverlay,
    slot: ReactiveStateSlot,
) {
    debug_assert_not_fragment(node, "state_overlay");
    if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
        binding.set_state_overlay(overlay, slot);
        return;
    }
    let mut binding = StyleBinding::default();
    binding.set_state_overlay(overlay, slot);
    let _previous = node.attach_untracked(binding);
}

pub(super) fn install_class(node: &mut Node, source: ClassSource) {
    debug_assert_not_fragment(node, "class");
    let incoming = binding_from_source(source);
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        let existing = binding.all_atoms();
        let incoming_atoms = incoming.all_atoms();
        binding.note_user(SlotSet::of(&incoming_atoms));
        let merged = merge_classes(&existing, &incoming_atoms, 0);
        binding.replace_owned(merged.into_iter().collect());
        return;
    }
    let _previous = node.attach(incoming);
}

fn debug_assert_not_fragment(node: &Node, op: &'static str) {
    debug_assert!(
        !matches!(node.kind, crate::kernel::NodeType::Fragment),
        "{op}() on a Fragment: fragments have no paint box"
    );
}

pub(super) fn install_theme(node: &mut Node, theme: Theme) {
    debug_assert_not_fragment(node, "theme");
    let _previous = node.attach(theme);
    crate::scene::emit_cache::theme_changed();
}

pub(super) fn install_focus_ring(node: &mut Node, ring: FocusRing) {
    debug_assert_not_fragment(node, "focus_ring");
    let _previous = node.attach(ring);
}

pub(super) fn install_reactive<F>(node: &mut Node, f: F)
where
    F: Fn() -> &'static [StyleAtom] + 'static,
{
    debug_assert_not_fragment(node, "style_reactive");
    let reactive = StyleReactiveEffect::new();
    let atoms = reactive.atoms();
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.add_reactive(atoms);
    } else {
        let _previous = node.attach(StyleBinding::reactive_atoms(atoms));
    }

    let effect_state = reactive.clone();
    let current = reactive.atoms();
    let tag = node.tag();
    crate::reactive::effect_named("sigil-style::style_reactive", move || {
        let next = f();
        let previous = current.get();
        effect_state.set_slice(next);
        if let Some(tag) = tag
            && previous != next
        {
            let layout = previous
                .iter()
                .chain(next)
                .any(|atom| atom.affects_layout());
            tag.record(DeltaKind::layout_if(layout));
        }
    });
    let _previous = node.attach(reactive);
}

fn binding_from_source(source: ClassSource) -> StyleBinding {
    match source {
        ClassSource::Static(atoms) => StyleBinding::static_atoms(atoms),
        ClassSource::Owned(atoms) => StyleBinding::owned_atoms(*atoms),
        ClassSource::RawString(input) => StyleBinding::static_atoms(parse_classes(input)),
    }
}
