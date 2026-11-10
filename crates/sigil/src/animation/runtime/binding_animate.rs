use crate::animation::runtime::Animator;
use crate::reactive::Signal;
use crate::style::{Color, ColorAtomKind, NumericAtomKind, StyleAtom};

pub(crate) struct AttachedAnimator {
    pub(crate) kind: NumericAtomKind,
    pub(crate) signal: Signal<f32>,
    pub(crate) animator: Animator,
}

pub(crate) struct AttachedColorAnimator {
    pub(crate) kind: ColorAtomKind,
    pub(crate) channels: [Signal<f32>; 4],
    pub(crate) animators: [Animator; 4],
    pub(crate) last_target: Color,
}

pub(crate) fn find_numeric(kind: NumericAtomKind, atoms: &[StyleAtom]) -> Option<f32> {
    atoms.iter().rev().find_map(|atom| match (*atom, kind) {
        (StyleAtom::Opacity(v), NumericAtomKind::Opacity) => Some(f32::from(v)),
        (StyleAtom::BorderRadius(px), NumericAtomKind::BorderRadius)
        | (StyleAtom::FontSize(px), NumericAtomKind::FontSize)
        | (StyleAtom::LineHeight(px), NumericAtomKind::LineHeight)
        | (StyleAtom::Gap(px), NumericAtomKind::Gap)
        | (StyleAtom::PaddingX(px), NumericAtomKind::PaddingX)
        | (StyleAtom::PaddingY(px), NumericAtomKind::PaddingY) => Some(f32::from(px)),
        (StyleAtom::Width(kind, px), NumericAtomKind::Width)
        | (StyleAtom::Height(kind, px), NumericAtomKind::Height)
            if kind == crate::style::atom::ATOM_PX =>
        {
            Some(f32::from(px))
        }
        _ => None,
    })
}
