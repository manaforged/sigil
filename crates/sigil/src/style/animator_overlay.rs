use crate::style::{Rgba8, StyleAtom};
use ahash::AHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NumericAtomKind {
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
    TranslateXFraction,
    TranslateYFraction,
    RotationX,
    RotationY,
    TranslateZ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ColorAtomKind {
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

    pub(crate) const ALL: [Self; 16] = [
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
        Self::TranslateXFraction,
        Self::TranslateYFraction,
    ];

    pub(crate) const SPACE: [Self; 3] = [Self::RotationX, Self::RotationY, Self::TranslateZ];
}

impl ColorAtomKind {
    pub(crate) const ALL: [Self; 3] = [Self::Background, Self::Text, Self::Border];
}

pub(crate) trait AnimatorOverlay {
    fn value_for(&self, kind: NumericAtomKind) -> Option<f32>;

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Rgba8>;

    fn visit_layout_values(&self, visit: &mut dyn FnMut(NumericAtomKind, f32)) {
        for kind in NumericAtomKind::ALL
            .into_iter()
            .filter(|kind| kind.affects_layout())
        {
            if let Some(value) = self.value_for(kind) {
                visit(kind, value);
            }
        }
    }

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
        for kind in NumericAtomKind::ALL.into_iter().chain(NumericAtomKind::SPACE) {
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
