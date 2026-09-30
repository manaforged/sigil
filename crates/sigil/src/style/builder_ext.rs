use super::install::{
    install_animation, install_class, install_focus_ring, install_reactive, install_state_overlay,
    install_theme, install_tone,
};
use super::presence::install_presence;
use crate::animation_runtime::AnimationConfig;
use crate::kernel::{NodeBuilder, TextBuilder};
use crate::reactive::Signal;
use smallvec::SmallVec;

use std::cell::Cell;
use std::rc::Rc;

use crate::style::class_parse::intern_atoms;
use crate::style::styled::StateStyleBuilder;
use crate::style::{
    FocusRing, NumericAtomKind, ReactiveStateSlot, StateSlot, Style, StyleAtom, StyleBinding,
    StyleOverlay, StyleState, StyleVariant, Theme, Tone, parse_classes,
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

pub trait NodeStyleExt {
    #[must_use]
    fn theme(self, theme: Theme) -> Self;

    #[must_use]
    fn tone(self, tone: Tone) -> Self;

    #[must_use]
    fn focus_ring(self, ring: FocusRing) -> Self;

    #[must_use]
    fn presence(self, shown: Signal<bool>, config: AnimationConfig) -> Self;

    #[must_use]
    fn animate(self, kind: NumericAtomKind, value: Signal<f32>) -> Self;

    #[must_use]
    fn visible_when(self, signal: Signal<bool>) -> Self;

    #[must_use]
    fn paint_when<F>(self, signal: Signal<bool>, paint: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder;
}

macro_rules! style_sinks {
    ($builder:ty) => {
        impl $builder {
            #[must_use]
            pub(crate) fn class(self, source: impl Into<ClassSource>) -> Self {
                let source = source.into();
                self.flush_paint()
                    .mutate(move |node| install_class(node, source))
            }

            #[must_use]
            pub(crate) fn style_reactive<F>(self, f: F) -> Self
            where
                F: Fn() -> &'static [StyleAtom] + 'static,
            {
                self.flush_paint()
                    .mutate(move |node| install_reactive(node, f))
            }

            #[must_use]
            pub(crate) fn flush_paint(mut self) -> Self {
                let atoms = self.pending_style().take_atoms();
                if atoms.is_empty() {
                    return self;
                }
                self.mutate(move |node| install_class(node, ClassSource::Owned(Box::new(atoms))))
            }
        }

        impl NodeStyleExt for $builder {
            fn theme(self, theme: Theme) -> Self {
                self.mutate(move |node| install_theme(node, theme))
            }

            fn focus_ring(self, ring: FocusRing) -> Self {
                self.mutate(move |node| install_focus_ring(node, ring))
            }

            fn presence(self, shown: Signal<bool>, config: AnimationConfig) -> Self {
                self.flush_paint()
                    .mutate(move |node| install_presence(node, shown, config))
            }

            fn animate(self, kind: NumericAtomKind, value: Signal<f32>) -> Self {
                self.mutate(move |node| install_animation(node, kind, value))
            }

            fn tone(self, tone: Tone) -> Self {
                self.mutate(move |node| install_tone(node, tone))
            }

            fn paint_when<F>(self, signal: Signal<bool>, paint: F) -> Self
            where
                F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
            {
                let atoms = intern_atoms(paint(StateStyleBuilder::new()).into_atoms());
                self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
            }

            fn visible_when(self, signal: Signal<bool>) -> Self {
                let hidden = parse_classes("hidden");
                self.style_reactive(move || if signal.get() { EMPTY_ATOMS } else { hidden })
            }
        }
    };
}

style_sinks!(NodeBuilder);
style_sinks!(TextBuilder);

impl NodeBuilder {
    #[must_use]
    pub(crate) fn style(self, style: Style) -> Self {
        crate::style::styled::node::attach_style(self.flush_paint(), style)
    }

    pub(crate) fn seal_style(self) -> Self {
        self.mutate(|node| {
            if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
                binding.seal();
            }
        })
    }

    #[must_use]
    pub(crate) fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.flush_paint()
            .mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    #[must_use]
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

    #[must_use]
    pub(crate) fn class_when(self, signal: Signal<bool>, class: &'static str) -> Self {
        let atoms = parse_classes(class);
        self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
    }
}
