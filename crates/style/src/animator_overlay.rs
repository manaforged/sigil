use crate::{Rgba8, StateSlot, StyleAtom};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAtomKind {
    Opacity,
    BorderRadius,
    FontSize,
    LineHeight,
    Gap,
    PaddingX,
    PaddingY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorAtomKind {
    Background,
    Text,
}

pub trait AnimatorOverlay {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32>;

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8>;

    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]);
}
