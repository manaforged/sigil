use crate::style::atom::OPACITY_UNITS;
use crate::style::{Color, StyleAtom};
use ahash::AHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NumericAtomKind {
    /// Opacity from 0.0, clear, to 1.0, opaque.
    Opacity,
    /// Corner radius in logical pixels, 0 or more, rounded to a whole pixel.
    BorderRadius,
    /// Font size in logical pixels, 1 or more.
    FontSize,
    /// Line height in logical pixels, 1 or more.
    LineHeight,
    /// Space between children in logical pixels.
    Gap,
    /// Left and right padding in logical pixels.
    PaddingX,
    /// Top and bottom padding in logical pixels.
    PaddingY,
    /// Width in logical pixels.
    Width,
    /// Height in logical pixels.
    Height,
    /// Horizontal offset in logical pixels, positive to the right.
    TranslateX,
    /// Vertical offset in logical pixels, positive down.
    TranslateY,
    /// Horizontal scale as a factor, 1.0 unchanged.
    ScaleX,
    /// Vertical scale as a factor, 1.0 unchanged.
    ScaleY,
    /// Rotation in the plane of the screen in radians, 0.0 unturned.
    Rotation,
    /// Horizontal offset as a fraction of the node's width, from -16.0 to 16.0.
    TranslateXFraction,
    /// Vertical offset as a fraction of the node's height, from -16.0 to 16.0.
    TranslateYFraction,
    /// Rotation around the x axis in radians, 0.0 unturned.
    RotationX,
    /// Rotation around the y axis in radians, 0.0 unturned.
    RotationY,
    /// Rotation around the z axis in radians, after the x and y turns.
    RotationZ,
    /// Offset toward the viewer in logical pixels.
    TranslateZ,
    /// Perspective distance in logical pixels, 1 or more; the node must already set a perspective.
    Perspective,
    /// The `blur()` filter's standard deviation in logical pixels, 0 or more.
    Blur,
    /// The `brightness()` filter amount, 0 or more, 1.0 unchanged.
    Brightness,
    /// The `contrast()` filter amount, 0 or more, 1.0 unchanged.
    Contrast,
    /// The `grayscale()` filter amount, 0 or more, 0.0 unchanged.
    Grayscale,
    /// The `hue-rotate()` filter angle in degrees, rounded to a whole degree.
    HueRotate,
    /// The `invert()` filter amount, 0 or more, 0.0 unchanged.
    Invert,
    /// The `saturate()` filter amount, 0 or more, 1.0 unchanged.
    Saturate,
    /// The `sepia()` filter amount, 0 or more, 0.0 unchanged.
    Sepia,
    /// The `drop-shadow()` filter horizontal offset in logical pixels.
    DropShadowX,
    /// The `drop-shadow()` filter vertical offset in logical pixels.
    DropShadowY,
    /// The `drop-shadow()` filter blur in logical pixels, 0 or more.
    DropShadowBlur,
    /// The text shadow horizontal offset in logical pixels; the node must have at most one text shadow.
    TextShadowX,
    /// The text shadow vertical offset in logical pixels; the node must have at most one text shadow.
    TextShadowY,
    /// The text shadow blur in logical pixels, 0 or more; the node must have at most one text shadow.
    TextShadowBlur,
    /// Where the first gradient color sits, in percent from 0 to 100; the node must paint a gradient.
    StopFrom,
    /// Where the middle gradient color sits, in percent from 0 to 100; the gradient must have a middle color.
    StopVia,
    /// Where the last gradient color sits, in percent from 0 to 100; the node must paint a gradient.
    StopTo,
    /// Where a gradient mask starts to clear, in percent from 0 to 100; the node must have a gradient mask.
    MaskFrom,
    /// Where a gradient mask is fully clear, in percent from 0 to 100; the node must have a gradient mask.
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

    pub(crate) const fn affects_transform(self) -> bool {
        matches!(
            self,
            Self::TranslateX
                | Self::TranslateY
                | Self::ScaleX
                | Self::ScaleY
                | Self::Rotation
                | Self::TranslateXFraction
                | Self::TranslateYFraction
                | Self::RotationX
                | Self::RotationY
                | Self::RotationZ
                | Self::TranslateZ
                | Self::Perspective
        )
    }

    pub(crate) const fn initial(self) -> f32 {
        match self {
            Self::Opacity => OPACITY_UNITS,
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

    fn value_for_color(&self, kind: ColorAtomKind) -> Option<Color>;

    fn visit_layout_values(&self, visit: &mut dyn FnMut(NumericAtomKind, f32)) {
        default_visit_layout_values(self, visit);
    }

    fn update(
        &self,
        _atoms: &[StyleAtom],
        _theme: &crate::style::Theme,
        _tone: crate::style::Tone,
        _inherited_text: Color,
    ) {
    }

    fn cache_key(&self) -> u64 {
        default_cache_key(self)
    }
}

pub(crate) fn default_visit_layout_values<T: AnimatorOverlay + ?Sized>(
    overlay: &T,
    visit: &mut dyn FnMut(NumericAtomKind, f32),
) {
    for kind in NumericAtomKind::ALL
        .into_iter()
        .filter(|kind| kind.affects_layout())
    {
        if let Some(value) = overlay.value_for(kind) {
            visit(kind, value);
        }
    }
}

pub(crate) fn default_cache_key<T: AnimatorOverlay + ?Sized>(overlay: &T) -> u64 {
    let mut hasher = AHasher::default();
    let mut active = false;
    for kind in NumericAtomKind::ALL {
        if let Some(value) = overlay.value_for(kind) {
            kind.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
            active = true;
        }
    }
    for kind in ColorAtomKind::ALL {
        if let Some(color) = overlay.value_for_color(kind) {
            kind.hash(&mut hasher);
            color.hash(&mut hasher);
            active = true;
        }
    }
    if active { hasher.finish() } else { 0 }
}
