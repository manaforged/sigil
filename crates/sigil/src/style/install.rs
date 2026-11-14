use crate::animation::Property;
use crate::kernel::{DeltaKind, Node, NodeTag};
use crate::reactive::{Reactive, Runtime, Signal, effect_named, memo, untrack};

use crate::style::merge::SlotSet;
use crate::style::{
    ClassSource, FocusRing, ReactiveStateSlot, StyleAtom, StyleBinding, StyleOverlay,
    StyleReactiveEffect, Theme, Tone, ToneContext, merge_classes, parse_classes,
};

pub(super) fn install_animation(
    node: &mut Node,
    property: fn(f32) -> Property,
    value: Signal<f32>,
) {
    let Some(kind) = property(0.0).kind() else {
        property(0.0).refuse("Widget::drive");
        return;
    };
    let mut last = 0.0_f32;
    let value = memo(move || {
        let next = value.get();
        if next.is_finite() {
            last = next;
        } else {
            tracing::warn!(target: "sigil::animation", value = next, "ignored a non-finite driven style value");
        }
        last
    });
    if let Some(binding) = node.extension_mut::<StyleBinding>() {
        binding.bind_animation(property, value);
    } else {
        let mut binding = StyleBinding::default();
        binding.bind_animation(property, value);
        let _previous = node.attach(binding);
    }
    let tag = node.tag();
    let delta = DeltaKind::layout_if(kind.affects_layout());
    effect_named("sigil-style::animation", move || {
        let _ = value.get();
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
    mut overlay: StyleOverlay,
    slot: ReactiveStateSlot,
) {
    debug_assert_not_fragment(node, "state_overlay");
    let live = overlay.take_live();
    let tag = node.tag();
    if node.get_extension_mut_untracked::<StyleBinding>().is_none() {
        let _previous = node.attach_untracked(StyleBinding::default());
    }
    let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() else {
        return;
    };
    binding.set_state_overlay(overlay, slot);
    for (state, paint) in live.into_each() {
        for read in paint.reads() {
            let reactive = StyleReactiveEffect::new();
            binding.add_state_reactive(state, reactive.atoms());
            drive_reactive(tag, &reactive, read);
        }
    }
}

pub(super) fn install_class(node: &mut Node, source: ClassSource) {
    debug_assert_not_fragment(node, "class");
    let incoming = binding_from_source(source);
    if let Some(binding) = node.extension_mut::<StyleBinding>() {
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

pub(super) fn install_theme(node: &mut Node, theme: impl Reactive<Theme>) {
    debug_assert_not_fragment(node, "theme");
    let initial = untrack(|| theme.read());
    attach_theme(node, initial.clone());
    let Some(tag) = node.tag() else {
        return;
    };
    let mut last = initial;
    effect_named("sigil-style::theme", move || {
        let next = theme.read();
        if next != last {
            last = next.clone();
            tag.queue_edit(move |target| attach_theme(target, next));
            tag.record(DeltaKind::Layout);
            Runtime::mark_frame_dirty();
        }
    });
}

fn attach_theme(node: &mut Node, theme: Theme) {
    let _previous = node.attach(theme);
    crate::scene::emit_cache::theme_changed();
}

pub(super) fn install_focus_ring(node: &mut Node, ring: FocusRing) {
    debug_assert_not_fragment(node, "focus_ring");
    if let Some(current) = node.extension_mut::<FocusRing>() {
        current.merge(ring);
    } else {
        let _previous = node.attach(ring);
    }
}

pub(super) fn install_default_focus_ring(node: &mut Node, visible: Signal<bool>) {
    debug_assert_not_fragment(node, "focus_ring");
    if let Some(current) = node.extension_mut::<FocusRing>() {
        current.set_default_visible(visible);
    } else {
        let _previous = node.attach(FocusRing::from(visible));
    }
}

pub(super) fn install_reactive<F>(node: &mut Node, f: F)
where
    F: Fn() -> &'static [StyleAtom] + 'static,
{
    debug_assert_not_fragment(node, "style_reactive");
    let reactive = StyleReactiveEffect::new();
    let atoms = reactive.atoms();
    if let Some(binding) = node.extension_mut::<StyleBinding>() {
        binding.add_reactive(atoms);
    } else {
        let _previous = node.attach(StyleBinding::reactive_atoms(atoms));
    }
    drive_reactive(node.tag(), &reactive, f);
    let _previous = node.attach(reactive);
}

fn drive_reactive<F>(tag: Option<NodeTag>, reactive: &StyleReactiveEffect, f: F)
where
    F: Fn() -> &'static [StyleAtom] + 'static,
{
    let effect_state = reactive.clone();
    let current = reactive.atoms();
    effect_named("sigil-style::style_reactive", move || {
        let next = f();
        let previous = current.get();
        effect_state.set_slice(next);
        if let Some(tag) = tag
            && previous != next
        {
            if previous
                .iter()
                .chain(next)
                .copied()
                .any(StyleAtom::may_transform)
            {
                crate::style::binding::bump_transform_generation();
            }
            let layout = previous
                .iter()
                .chain(next)
                .any(|atom| atom.affects_layout());
            tag.record(DeltaKind::layout_if(layout));
        }
    });
}

fn binding_from_source(source: ClassSource) -> StyleBinding {
    match source {
        ClassSource::Static(atoms) => StyleBinding::static_atoms(atoms),
        ClassSource::Owned(atoms) => StyleBinding::owned_atoms(*atoms),
        ClassSource::RawString(input) => StyleBinding::static_atoms(parse_classes(input)),
    }
}
