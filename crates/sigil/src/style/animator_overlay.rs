use crate::style::{Rgba8, StateSlot, StyleAtom};
use ahash::AHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAtomKind {
    Opacity,
    BorderRadius,
    FontSize,
    LineHeight,
    Gap,
    PaddingX,
    PaddingY,
    Width,
    Height,
    TranslateX,
    TranslateY,
    ScaleX,
    ScaleY,
    Rotation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorAtomKind {
    Background,
    Text,
    Border,
}

impl NumericAtomKind {
    pub(crate) const fn affects_layout(self) -> bool {
        matches!(
            self,
            Self::FontSize
                | Self::LineHeight
                | Self::Gap
                | Self::PaddingX
                | Self::PaddingY
                | Self::Width
                | Self::Height
        )
    }

    pub(crate) const ALL: [Self; 14] = [
        Self::Opacity,
        Self::BorderRadius,
        Self::FontSize,
        Self::LineHeight,
        Self::Gap,
        Self::PaddingX,
        Self::PaddingY,
        Self::Width,
        Self::Height,
        Self::TranslateX,
        Self::TranslateY,
        Self::ScaleX,
        Self::ScaleY,
        Self::Rotation,
    ];
}

impl ColorAtomKind {
    pub(crate) const ALL: [Self; 3] = [Self::Background, Self::Text, Self::Border];
}

pub trait AnimatorOverlay {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32>;

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8>;

    fn retarget_to_slot(&mut self, slot: StateSlot, base: &[StyleAtom]);

    fn update(
        &self,
        _atoms: &[StyleAtom],
        _theme: &crate::style::Theme,
        _tone: crate::style::Tone,
        _inherited_text: Rgba8,
    ) {
    }

    fn cache_key(&self) -> u64 {
        let mut hasher = AHasher::default();
        for kind in NumericAtomKind::ALL {
            if let Some(value) = self.value_for(kind) {
                kind.hash(&mut hasher);
                value.to_bits().hash(&mut hasher);
            }
        }
        for kind in ColorAtomKind::ALL {
            if let Some(color) = self.value_for_color(kind) {
                kind.hash(&mut hasher);
                color.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}
