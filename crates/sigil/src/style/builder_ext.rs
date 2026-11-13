use super::install::{
    install_animation, install_class, install_focus_ring, install_reactive, install_state_overlay,
    install_theme, install_tone,
};
use super::presence::install_presence;
use crate::animation::Property;
use crate::animation::runtime::AnimationConfig;
use crate::kernel::{NodeBuilder, TextBuilder};
use crate::reactive::{Reactive, Signal};
use crate::widgets::Widget;
use smallvec::SmallVec;

use std::cell::Cell;
use std::rc::Rc;

use crate::style::class_parse::intern_atoms;
use crate::style::styled::StateStyleBuilder;
use crate::style::{
    FocusRing, ReactiveStateSlot, StateSlot, Style, StyleAtom, StyleBinding, StyleOverlay,
    StyleState, StyleVariant, Theme, Tone, parse_classes,
};

static EMPTY_ATOMS: &[StyleAtom] = &[];

pub(crate) enum ClassSource {
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

impl NodeBuilder {
    pub(crate) fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.flush_paint()
            .mutate(move |node| install_class(node, source))
    }

    pub(crate) fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.flush_paint()
            .mutate(move |node| install_reactive(node, f))
    }

    pub(crate) fn flush_paint(mut self) -> Self {
        let atoms = self.pending_style().take_atoms();
        let images = self.pending_style().take_images();
        let this = if images.is_empty() {
            self
        } else {
            self.mutate(move |node| images.install(node))
        };
        if atoms.is_empty() {
            return this;
        }
        this.mutate(move |node| install_class(node, ClassSource::Owned(Box::new(atoms))))
    }
}

impl TextBuilder {
    pub(crate) fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.map_node(move |node| node.class(source))
    }

    pub(crate) fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.map_node(move |node| node.style_reactive(f))
    }
}

pub(crate) fn theme(node: NodeBuilder, theme: Theme) -> NodeBuilder {
    node.mutate(move |node| install_theme(node, theme))
}

pub(crate) fn tone(node: NodeBuilder, tone: Tone) -> NodeBuilder {
    node.mutate(move |node| install_tone(node, tone))
}

pub(crate) fn focus_ring(node: NodeBuilder, ring: FocusRing) -> NodeBuilder {
    node.mutate(move |node| install_focus_ring(node, ring))
}

pub(crate) fn presence(
    node: NodeBuilder,
    shown: Signal<bool>,
    config: AnimationConfig,
) -> NodeBuilder {
    node.flush_paint()
        .mutate(move |node| install_presence(node, shown, config))
}

pub(crate) fn drive(
    node: NodeBuilder,
    property: fn(f32) -> Property,
    value: Signal<f32>,
) -> NodeBuilder {
    node.mutate(move |node| install_animation(node, property, value))
}

pub(crate) fn paint_when(
    node: NodeBuilder,
    active: impl Reactive<bool> + 'static,
    paint: StateStyleBuilder,
) -> NodeBuilder {
    let (atoms, images) = paint.into_parts();
    let atoms = intern_atoms(atoms);
    let node = if images.is_empty() {
        node
    } else {
        node.mutate(move |node| images.install(node))
    };
    node.style_reactive(move || if active.read() { atoms } else { EMPTY_ATOMS })
}

pub(crate) fn visible(node: NodeBuilder, shown: impl Reactive<bool> + 'static) -> NodeBuilder {
    let hidden = parse_classes("hidden");
    node.style_reactive(move || if shown.read() { EMPTY_ATOMS } else { hidden })
}

impl NodeBuilder {
    pub(crate) fn style(self, mut style: Style) -> Self {
        let mut this = self.flush_paint();
        if let Some(base) = this.take_style() {
            style.merge_under(&base);
        }
        crate::style::styled::node::attach_style(this, style)
    }

    pub(crate) fn seal_style(self) -> Self {
        self.mutate(|node| {
            if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
                binding.seal();
            }
        })
    }

    pub(crate) fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.flush_paint()
            .mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    pub(crate) fn stateful_style<F>(
        self,
        variant: StyleVariant,
        overlay: StyleOverlay,
        state_fn: F,
    ) -> Self
    where
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

    pub(crate) fn class_when(self, signal: Signal<bool>, class: &'static str) -> Self {
        let atoms = parse_classes(class);
        self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
    }
}
