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
    RotationZ,
    TranslateZ,
    Perspective,
    Blur,
    Brightness,
    Contrast,
    Grayscale,
    HueRotate,
    Invert,
    Saturate,
    Sepia,
    DropShadowX,
    DropShadowY,
    DropShadowBlur,
    TextShadowX,
    TextShadowY,
    TextShadowBlur,
    StopFrom,
    StopVia,
    StopTo,
    MaskFrom,
    MaskTo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ColorAtomKind {
    Background,
    Text,
    Border,
    DropShadow,
    TextShadow,
    StopFrom,
    StopVia,
    StopTo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    Plain,
    Filter,
    TextShadow,
    Gradient,
    Mask,
}

impl NumericAtomKind {
    pub(crate) const COUNT: usize = 40;

    pub(crate) const ALL: [Self; Self::COUNT] = [
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
        Self::RotationX,
        Self::RotationY,
        Self::RotationZ,
        Self::TranslateZ,
        Self::Perspective,
        Self::Blur,
        Self::Brightness,
        Self::Contrast,
        Self::Grayscale,
        Self::HueRotate,
        Self::Invert,
        Self::Saturate,
        Self::Sepia,
        Self::DropShadowX,
        Self::DropShadowY,
        Self::DropShadowBlur,
        Self::TextShadowX,
        Self::TextShadowY,
        Self::TextShadowBlur,
        Self::StopFrom,
        Self::StopVia,
        Self::StopTo,
        Self::MaskFrom,
        Self::MaskTo,
    ];

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

    pub(crate) const fn initial(self) -> f32 {
        match self {
            Self::Opacity => 10_000.0,
            Self::ScaleX | Self::ScaleY | Self::Brightness | Self::Contrast | Self::Saturate => 1.0,
            Self::StopVia => 50.0,
            Self::StopTo | Self::MaskTo => 100.0,
            _ => 0.0,
        }
    }

    pub(crate) const fn family(self) -> Family {
        match self {
            Self::Blur
            | Self::Brightness
            | Self::Contrast
            | Self::Grayscale
            | Self::HueRotate
            | Self::Invert
            | Self::Saturate
            | Self::Sepia
            | Self::DropShadowX
            | Self::DropShadowY
            | Self::DropShadowBlur => Family::Filter,
            Self::TextShadowX | Self::TextShadowY | Self::TextShadowBlur => Family::TextShadow,
            Self::StopFrom | Self::StopVia | Self::StopTo => Family::Gradient,
            Self::MaskFrom | Self::MaskTo => Family::Mask,
            _ => Family::Plain,
        }
    }
}

impl ColorAtomKind {
    pub(crate) const COUNT: usize = 8;

    pub(crate) const ALL: [Self; Self::COUNT] = [
        Self::Background,
        Self::Text,
        Self::Border,
        Self::DropShadow,
        Self::TextShadow,
        Self::StopFrom,
        Self::StopVia,
        Self::StopTo,
    ];

    pub(crate) const fn family(self) -> Family {
        match self {
            Self::Background | Self::Text | Self::Border => Family::Plain,
            Self::DropShadow => Family::Filter,
            Self::TextShadow => Family::TextShadow,
            Self::StopFrom | Self::StopVia | Self::StopTo => Family::Gradient,
        }
    }
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
