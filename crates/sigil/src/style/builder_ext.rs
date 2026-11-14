use super::install::{
    install_animation, install_class, install_default_focus_ring, install_focus_ring,
    install_reactive, install_state_overlay, install_theme, install_tone,
};
use super::presence::{Edge, install_motion, install_presence};
use crate::animation::{Property, Transition};
use crate::kernel::{NodeBuilder, TextBuilder};
use crate::reactive::{Reactive, Signal};
use crate::widgets::Widget;
use smallvec::SmallVec;

use std::cell::Cell;
use std::rc::Rc;

use crate::style::class_parse::intern_atoms;
use crate::style::styled::StateStyleBuilder;
use crate::style::{
    FocusRing, Paint, ReactiveStateSlot, StateSlot, Style, StyleAtom, StyleBinding, StyleOverlay,
    StyleState, StyleVariant, Theme, Tone, parse_classes,
};

pub(crate) static EMPTY_ATOMS: &[StyleAtom] = &[];

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
        let pending = self.pending_style();
        let atoms = pending.take_atoms();
        let images = pending.take_images();
        let shaders = pending.take_shaders();
        let live = pending.take_live();
        let this = if images.is_empty() && shaders.is_empty() {
            self
        } else {
            self.mutate(move |node| {
                images.install(node);
                shaders.install(node);
            })
        };
        let this = live.install(this);
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

pub(crate) fn theme(node: NodeBuilder, theme: impl Reactive<Theme>) -> NodeBuilder {
    node.mutate(move |node| install_theme(node, theme))
}

pub(crate) fn tone(node: NodeBuilder, tone: Tone) -> NodeBuilder {
    node.mutate(move |node| install_tone(node, tone))
}

pub(crate) fn focus_ring(node: NodeBuilder, ring: FocusRing) -> NodeBuilder {
    node.mutate(move |node| install_focus_ring(node, ring))
}

pub(crate) fn default_focus_ring(node: NodeBuilder, visible: Signal<bool>) -> NodeBuilder {
    node.mutate(move |node| install_default_focus_ring(node, visible))
}

pub(crate) fn presence(
    node: NodeBuilder,
    shown: Signal<bool>,
    transition: Transition,
) -> NodeBuilder {
    node.transition(transition)
        .flush_paint()
        .mutate(move |node| install_presence(node, shown))
}

pub(crate) fn motion(node: NodeBuilder, edge: Edge, property: Property) -> NodeBuilder {
    node.flush_paint()
        .mutate(move |node| install_motion(node, edge, property))
}

pub(crate) fn drive(
    node: NodeBuilder,
    property: fn(f32) -> Property,
    value: impl Reactive<f32>,
) -> NodeBuilder {
    let value = value.into_signal();
    node.mutate(move |node| install_animation(node, property, value))
}

pub(crate) fn paint_when(
    node: NodeBuilder,
    active: impl Reactive<bool> + 'static,
    paint: StateStyleBuilder,
) -> NodeBuilder {
    let (atoms, images, live) = paint.into_parts();
    let atoms = intern_atoms(atoms);
    let active = active.into_signal();
    let node = if images.is_empty() {
        node
    } else {
        node.mutate(move |node| images.install(node))
    };
    let node = node.style_reactive(move || if active.get() { atoms } else { EMPTY_ATOMS });
    live.install_when(node, active)
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
        self.flush_paint().mutate(|node| {
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
        let restacks = overlay.any(StyleAtom::restacks);
        self.class(variant.base())
            .style_reactive(move || {
                let state = state_fn();
                let slot = state.slot();
                if cell_for_closure.replace(slot) != slot && restacks {
                    crate::style::binding::bump_hit_generation();
                }
                variant.state(state)
            })
            .state_overlay(overlay, slot_cell)
    }

    pub(crate) fn class_when(self, signal: Signal<bool>, class: &'static str) -> Self {
        let atoms = parse_classes(class);
        self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
    }
}
