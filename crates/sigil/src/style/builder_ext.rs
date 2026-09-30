use super::presence::install_presence;
use crate::animation_runtime::AnimationConfig;
use crate::kernel::{Node, NodeBuilder, TextBuilder};
use crate::reactive::{Runtime, Signal, effect_named};
use smallvec::SmallVec;

use std::cell::Cell;
use std::rc::Rc;

use crate::style::merge::SlotSet;
use crate::style::{
    FocusRing, NumericAtomKind, ReactiveStateSlot, StateSlot, Style, StyleAtom, StyleBinding,
    StyleOverlay, StyleReactiveEffect, StyleState, StyleVariant, Theme, Tone, ToneContext,
    merge_classes, parse_classes,
};

static EMPTY_ATOMS: &[StyleAtom] = &[];

pub enum ClassSource {
    Static(&'static [StyleAtom]),
    Owned(Box<SmallVec<[StyleAtom; 16]>>),
    RawString(&'static str),
}

impl From<&'static [StyleAtom]> for ClassSource {
    fn from(value: &'static [StyleAtom]) -> Self {
        Self::Static(value)
    }
}

impl<const N: usize> From<&'static [StyleAtom; N]> for ClassSource {
    fn from(value: &'static [StyleAtom; N]) -> Self {
        Self::Static(value.as_slice())
    }
}

impl From<StyleAtom> for ClassSource {
    fn from(value: StyleAtom) -> Self {
        Self::Owned(Box::new(SmallVec::from_elem(value, 1)))
    }
}

impl<const N: usize> From<[StyleAtom; N]> for ClassSource {
    fn from(value: [StyleAtom; N]) -> Self {
        Self::Owned(Box::new(value.into_iter().collect()))
    }
}

impl From<&'static str> for ClassSource {
    fn from(value: &'static str) -> Self {
        Self::RawString(value)
    }
}

impl From<Style> for ClassSource {
    fn from(value: Style) -> Self {
        Self::Owned(Box::new(value.into_atoms()))
    }
}

pub trait NodeStyleExt {
    #[must_use]
    fn class(self, source: impl Into<ClassSource>) -> Self;

    #[must_use]
    fn style(self, style: Style) -> Self;

    #[must_use]
    fn theme(self, theme: Theme) -> Self;

    #[must_use]
    fn tone(self, tone: Tone) -> Self;

    #[must_use]
    fn focus_ring(self, ring: FocusRing) -> Self;

    #[must_use]
    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static;

    #[must_use]
    fn presence(self, shown: Signal<bool>, config: AnimationConfig) -> Self;

    #[must_use]
    fn animate(self, kind: NumericAtomKind, value: Signal<f32>) -> Self;

    #[must_use]
    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self;

    #[must_use]
    fn stateful_style<F>(self, variant: StyleVariant, overlay: StyleOverlay, state_fn: F) -> Self
    where
        Self: Sized,
        F: Fn() -> StyleState + 'static,
    {
        let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
        let cell_for_closure = Rc::clone(&slot_cell);
        self.class(variant.base())
            .style_reactive(move || {
                let state = state_fn();
                cell_for_closure.set(state.slot());
                variant.state(state)
            })
            .state_overlay(overlay, slot_cell)
    }

    #[must_use]
    fn class_when(self, signal: Signal<bool>, class: &'static str) -> Self
    where
        Self: Sized,
    {
        let atoms = parse_classes(class);
        self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
    }

    #[must_use]
    fn visible_when(self, signal: Signal<bool>) -> Self
    where
        Self: Sized,
    {
        let hidden = parse_classes("hidden");
        self.style_reactive(move || if signal.get() { EMPTY_ATOMS } else { hidden })
    }
}

impl NodeBuilder {
    pub(crate) fn seal_style(self) -> Self {
        self.mutate(|node| {
            if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
                binding.seal();
            }
        })
    }
}

impl NodeStyleExt for NodeBuilder {
    fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.mutate(move |node| install_class(node, source))
    }

    fn style(self, style: Style) -> Self {
        crate::style::styled::node::attach_style(self, style)
    }

    fn theme(self, theme: Theme) -> Self {
        self.mutate(move |node| install_theme(node, theme))
    }

    fn focus_ring(self, ring: FocusRing) -> Self {
        self.mutate(move |node| install_focus_ring(node, ring))
    }

    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.mutate(move |node| install_reactive(node, f))
    }

    fn presence(self, shown: Signal<bool>, config: AnimationConfig) -> Self {
        self.mutate(move |node| install_presence(node, shown, config))
    }

    fn animate(self, kind: NumericAtomKind, value: Signal<f32>) -> Self {
        self.mutate(move |node| install_animation(node, kind, value))
    }

    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    fn tone(self, tone: Tone) -> Self {
        self.mutate(move |node| install_tone(node, tone))
    }
}

impl NodeStyleExt for TextBuilder {
    fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.mutate(move |node| install_class(node, source))
    }

    fn style(self, style: Style) -> Self {
        debug_assert!(
            !style.has_state_overlays(),
            "text has no pointer state; put state styles on the container around it"
        );
        self.class(style)
    }

    fn theme(self, theme: Theme) -> Self {
        self.mutate(move |node| install_theme(node, theme))
    }

    fn focus_ring(self, ring: FocusRing) -> Self {
        self.mutate(move |node| install_focus_ring(node, ring))
    }

    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.mutate(move |node| install_reactive(node, f))
    }

    fn presence(self, shown: Signal<bool>, config: AnimationConfig) -> Self {
        self.mutate(move |node| install_presence(node, shown, config))
    }

    fn animate(self, kind: NumericAtomKind, value: Signal<f32>) -> Self {
        self.mutate(move |node| install_animation(node, kind, value))
    }

    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    fn tone(self, tone: Tone) -> Self {
        self.mutate(move |node| install_tone(node, tone))
    }
}

pub(super) fn install_animation(node: &mut Node, kind: NumericAtomKind, value: Signal<f32>) {
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.bind_animation(kind, value);
    } else {
        let mut binding = StyleBinding::default();
        binding.bind_animation(kind, value);
        let _previous = node.attach(binding);
    }
    effect_named("sigil-style::animation", move || {
        assert!(
            value.get().is_finite(),
            "animated style values must be finite"
        );
        Runtime::mark_frame_dirty();
    });
}

fn install_tone(node: &mut Node, tone: Tone) {
    debug_assert_not_fragment(node, "tone");
    let _previous = node.attach(ToneContext::new(tone));
}

fn install_state_overlay(node: &mut Node, overlay: StyleOverlay, slot: ReactiveStateSlot) {
    debug_assert_not_fragment(node, "state_overlay");
    if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
        binding.set_state_overlay(overlay, slot);
        return;
    }
    let mut binding = StyleBinding::default();
    binding.set_state_overlay(overlay, slot);
    let _previous = node.attach_untracked(binding);
}

fn install_class(node: &mut Node, source: ClassSource) {
    debug_assert_not_fragment(node, "class");
    let incoming = binding_from_source(source);
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        let existing = binding.all_atoms();
        let incoming_atoms = incoming.all_atoms();
        binding.note_user(SlotSet::of(&incoming_atoms));
        let merged = merge_classes(&existing, &incoming_atoms, 0);
        binding.replace_owned(smallvec16(merged));
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

fn install_theme(node: &mut Node, theme: Theme) {
    debug_assert_not_fragment(node, "theme");
    let _previous = node.attach(theme);
}

fn install_focus_ring(node: &mut Node, ring: FocusRing) {
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
    crate::reactive::effect_named("sigil-style::style_reactive", move || {
        effect_state.set_slice(f());
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

fn smallvec16(atoms: SmallVec<[StyleAtom; 32]>) -> SmallVec<[StyleAtom; 16]> {
    atoms.into_iter().collect()
}
