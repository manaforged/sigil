use crate::scene::Shadow;
use crate::style::atom::{OPACITY_UNITS, shadow_color};
use crate::style::{ColorAtomKind, ColorValue, Family, NumericAtomKind};

/// One animatable property and its value, set by a keyframe or driven from a signal.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Property {
    /// Opacity, from 0.0 to 1.0.
    Opacity(f32),
    /// Horizontal translation in pixels.
    TranslateX(f32),
    /// Vertical translation in pixels.
    TranslateY(f32),
    /// Horizontal translation in percent of the node's width.
    TranslateXPercent(f32),
    /// Vertical translation in percent of the node's height.
    TranslateYPercent(f32),
    /// Uniform scale factor.
    Scale(f32),
    /// Rotation in radians.
    Rotate(f32),
    /// Rotation around the x axis in radians.
    RotateX(f32),
    /// Rotation around the y axis in radians.
    RotateY(f32),
    /// Rotation around the z axis in radians, after the x and y turns.
    RotateZ(f32),
    /// Translation toward the viewer in pixels.
    TranslateZ(f32),
    /// The perspective distance in pixels that the node gives its children; the node must set one.
    Perspective(f32),
    /// Background color.
    Background(ColorValue),
    /// Text color.
    TextColor(ColorValue),
    /// Border color.
    BorderColor(ColorValue),
    /// The `blur()` filter's standard deviation in pixels.
    Blur(f32),
    /// The `brightness()` filter amount, 1.0 unchanged.
    Brightness(f32),
    /// The `contrast()` filter amount, 1.0 unchanged.
    Contrast(f32),
    /// The `grayscale()` filter amount, 0.0 unchanged.
    Grayscale(f32),
    /// The `hue-rotate()` filter angle in degrees.
    HueRotate(f32),
    /// The `invert()` filter amount, 0.0 unchanged.
    Invert(f32),
    /// The `saturate()` filter amount, 1.0 unchanged.
    Saturate(f32),
    /// The `sepia()` filter amount, 0.0 unchanged.
    Sepia(f32),
    /// The `drop-shadow()` filter offset, blur, and color.
    DropShadow(Shadow),
    /// The text shadow offset, blur, and color; a node with several text shadows keeps them.
    TextShadow(Shadow),
    /// The color of the first stop of the node's gradient.
    GradientFrom(ColorValue),
    /// The middle gradient color; the node's gradient must have a middle stop.
    GradientVia(ColorValue),
    /// The color of the last stop of the node's gradient.
    GradientTo(ColorValue),
    /// Where the first gradient color sits, in percent.
    GradientFromPosition(f32),
    /// Where the middle gradient color sits, in percent.
    GradientViaPosition(f32),
    /// Where the last gradient color sits, in percent.
    GradientToPosition(f32),
    /// Where a gradient mask starts to clear, in percent.
    MaskFrom(f32),
    /// Where a gradient mask is fully clear, in percent.
    MaskTo(f32),
    /// Horizontal scale factor, 1.0 unchanged.
    ScaleX(f32),
    /// Vertical scale factor, 1.0 unchanged.
    ScaleY(f32),
    /// Corner radius in pixels, rounded to a whole pixel.
    BorderRadius(f32),
    /// Font size in pixels, 1 or more; `Widget::drive` only, `Keyframes` ignores it with a warning.
    FontSize(f32),
    /// Line height in pixels, 1 or more; `Widget::drive` only, `Keyframes` ignores it with a warning.
    LineHeight(f32),
    /// Space between children in pixels; `Widget::drive` only, `Keyframes` ignores it with a warning.
    Gap(f32),
    /// Left and right padding in pixels; `Widget::drive` only, `Keyframes` ignores it with a warning.
    PaddingX(f32),
    /// Top and bottom padding in pixels; `Widget::drive` only, `Keyframes` ignores it with a warning.
    PaddingY(f32),
    /// Width in pixels; `Widget::drive` only, `Keyframes` ignores it with a warning.
    Width(f32),
    /// Height in pixels; `Widget::drive` only, `Keyframes` ignores it with a warning.
    Height(f32),
    /// The `drop-shadow()` filter horizontal offset in pixels.
    DropShadowX(f32),
    /// The `drop-shadow()` filter vertical offset in pixels.
    DropShadowY(f32),
    /// The `drop-shadow()` filter blur in pixels.
    DropShadowBlur(f32),
    /// The text shadow horizontal offset in pixels; the node must have at most one text shadow.
    TextShadowX(f32),
    /// The text shadow vertical offset in pixels; the node must have at most one text shadow.
    TextShadowY(f32),
    /// The text shadow blur in pixels; the node must have at most one text shadow.
    TextShadowBlur(f32),
    /// The `backdrop-filter` blur's standard deviation in pixels.
    BackdropBlur(f32),
    /// The `backdrop-filter` brightness amount, 1.0 unchanged.
    BackdropBrightness(f32),
    /// The `backdrop-filter` saturate amount, 1.0 unchanged.
    BackdropSaturate(f32),
}

pub(super) type Key = (u8, [u32; 3], Option<ColorValue>);

impl Property {
    pub(crate) fn numeric(self, kind: NumericAtomKind) -> Option<f32> {
        match self {
            Self::Scale(factor)
                if matches!(kind, NumericAtomKind::ScaleX | NumericAtomKind::ScaleY) =>
            {
                Some(factor)
            }
            Self::DropShadow(shadow) if kind.family() == Family::Filter => {
                shadow_part(shadow, kind)
            }
            Self::TextShadow(shadow) if kind.family() == Family::TextShadow => {
                shadow_part(shadow, kind)
            }
            _ => self
                .scalar()
                .filter(|(owner, _)| *owner == kind)
                .map(|(_, value)| value),
        }
        .filter(|value| value.is_finite())
    }

    pub(crate) fn kind(self) -> Option<NumericAtomKind> {
        self.scalar().map(|(kind, _)| kind)
    }

    pub(crate) fn layout(self) -> bool {
        self.kind().is_some_and(NumericAtomKind::affects_layout)
    }

    pub(crate) fn refuse(self, path: &'static str) {
        tracing::warn!(
            target: "sigil::animation",
            property = ?self,
            path,
            "this property cannot be animated on this path and was ignored"
        );
    }

    fn scalar(self) -> Option<(NumericAtomKind, f32)> {
        use NumericAtomKind as K;
        Some(match self {
            Self::Opacity(value) => (K::Opacity, value.clamp(0.0, 1.0) * OPACITY_UNITS),
            Self::TranslateX(px) => (K::TranslateX, px),
            Self::TranslateY(px) => (K::TranslateY, px),
            Self::TranslateXPercent(percent) => (K::TranslateXFraction, percent / 100.0),
            Self::TranslateYPercent(percent) => (K::TranslateYFraction, percent / 100.0),
            Self::Rotate(radians) => (K::Rotation, radians),
            Self::RotateX(radians) => (K::RotationX, radians),
            Self::RotateY(radians) => (K::RotationY, radians),
            Self::RotateZ(radians) => (K::RotationZ, radians),
            Self::TranslateZ(px) => (K::TranslateZ, px),
            Self::Perspective(px) => (K::Perspective, px.max(1.0)),
            Self::Blur(px) => (K::Blur, px.max(0.0)),
            Self::Brightness(amount) => (K::Brightness, amount.max(0.0)),
            Self::Contrast(amount) => (K::Contrast, amount.max(0.0)),
            Self::Grayscale(amount) => (K::Grayscale, amount.max(0.0)),
            Self::HueRotate(degrees) => (K::HueRotate, degrees),
            Self::Invert(amount) => (K::Invert, amount.max(0.0)),
            Self::Saturate(amount) => (K::Saturate, amount.max(0.0)),
            Self::Sepia(amount) => (K::Sepia, amount.max(0.0)),
            Self::GradientFromPosition(percent) => (K::StopFrom, percent.clamp(0.0, 100.0)),
            Self::GradientViaPosition(percent) => (K::StopVia, percent.clamp(0.0, 100.0)),
            Self::GradientToPosition(percent) => (K::StopTo, percent.clamp(0.0, 100.0)),
            Self::MaskFrom(percent) => (K::MaskFrom, percent.clamp(0.0, 100.0)),
            Self::MaskTo(percent) => (K::MaskTo, percent.clamp(0.0, 100.0)),
            Self::Scale(factor) | Self::ScaleX(factor) => (K::ScaleX, factor),
            Self::ScaleY(factor) => (K::ScaleY, factor),
            Self::BorderRadius(px) => (K::BorderRadius, px),
            Self::FontSize(px) => (K::FontSize, px),
            Self::LineHeight(px) => (K::LineHeight, px),
            Self::Gap(px) => (K::Gap, px),
            Self::PaddingX(px) => (K::PaddingX, px),
            Self::PaddingY(px) => (K::PaddingY, px),
            Self::Width(px) => (K::Width, px),
            Self::Height(px) => (K::Height, px),
            Self::DropShadowX(px) => (K::DropShadowX, px),
            Self::DropShadowY(px) => (K::DropShadowY, px),
            Self::DropShadowBlur(px) => (K::DropShadowBlur, px.max(0.0)),
            Self::TextShadowX(px) => (K::TextShadowX, px),
            Self::TextShadowY(px) => (K::TextShadowY, px),
            Self::TextShadowBlur(px) => (K::TextShadowBlur, px.max(0.0)),
            Self::BackdropBlur(px) => (K::BackdropBlur, px.max(0.0)),
            Self::BackdropBrightness(amount) => (K::BackdropBrightness, amount.max(0.0)),
            Self::BackdropSaturate(amount) => (K::BackdropSaturate, amount.max(0.0)),
            _ => return None,
        })
    }

    pub(crate) fn color(self, kind: ColorAtomKind) -> Option<ColorValue> {
        match (self, kind) {
            (Self::Background(color), ColorAtomKind::Background)
            | (Self::TextColor(color), ColorAtomKind::Text)
            | (Self::BorderColor(color), ColorAtomKind::Border)
            | (Self::GradientFrom(color), ColorAtomKind::StopFrom)
            | (Self::GradientVia(color), ColorAtomKind::StopVia)
            | (Self::GradientTo(color), ColorAtomKind::StopTo) => Some(color),
            (Self::DropShadow(shadow), ColorAtomKind::DropShadow)
            | (Self::TextShadow(shadow), ColorAtomKind::TextShadow) => {
                Some(shadow_color(shadow).into())
            }
            _ => None,
        }
    }

    pub(super) fn key(self) -> Key {
        let tag = self.tag();
        match self {
            Self::DropShadow(shadow) | Self::TextShadow(shadow) => {
                let [x, y] = shadow.offset;
                (
                    tag,
                    [bits(x), bits(y), bits(shadow.blur)],
                    Some(shadow_color(shadow).into()),
                )
            }
            Self::Background(color)
            | Self::TextColor(color)
            | Self::BorderColor(color)
            | Self::GradientFrom(color)
            | Self::GradientVia(color)
            | Self::GradientTo(color) => (tag, [0; 3], Some(color)),
            _ => {
                let value = self.scalar().map_or(0.0, |(_, value)| value);
                (tag, [bits(value), 0, 0], None)
            }
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Opacity(_) => 0,
            Self::TranslateX(_) => 1,
            Self::TranslateY(_) => 2,
            Self::TranslateXPercent(_) => 3,
            Self::TranslateYPercent(_) => 4,
            Self::Scale(_) => 5,
            Self::Rotate(_) => 6,
            Self::Background(_) => 7,
            Self::TextColor(_) => 8,
            Self::BorderColor(_) => 9,
            Self::RotateX(_) => 10,
            Self::RotateY(_) => 11,
            Self::TranslateZ(_) => 12,
            Self::RotateZ(_) => 13,
            Self::Perspective(_) => 14,
            Self::Blur(_) => 15,
            Self::Brightness(_) => 16,
            Self::Contrast(_) => 17,
            Self::Grayscale(_) => 18,
            Self::HueRotate(_) => 19,
            Self::Invert(_) => 20,
            Self::Saturate(_) => 21,
            Self::Sepia(_) => 22,
            Self::DropShadow(_) => 23,
            Self::TextShadow(_) => 24,
            Self::GradientFrom(_) => 25,
            Self::GradientVia(_) => 26,
            Self::GradientTo(_) => 27,
            Self::GradientFromPosition(_) => 28,
            Self::GradientViaPosition(_) => 29,
            Self::GradientToPosition(_) => 30,
            Self::MaskFrom(_) => 31,
            Self::MaskTo(_) => 32,
            other => other.driven_tag(),
        }
    }

    const fn driven_tag(self) -> u8 {
        match self {
            Self::ScaleX(_) => 33,
            Self::ScaleY(_) => 34,
            Self::BorderRadius(_) => 35,
            Self::FontSize(_) => 36,
            Self::LineHeight(_) => 37,
            Self::Gap(_) => 38,
            Self::PaddingX(_) => 39,
            Self::PaddingY(_) => 40,
            Self::Width(_) => 41,
            Self::Height(_) => 42,
            Self::DropShadowX(_) => 43,
            Self::DropShadowY(_) => 44,
            Self::DropShadowBlur(_) => 45,
            Self::TextShadowX(_) => 46,
            Self::TextShadowY(_) => 47,
            Self::TextShadowBlur(_) => 48,
            Self::BackdropBlur(_) => 49,
            Self::BackdropBrightness(_) => 50,
            Self::BackdropSaturate(_) => 51,
            _ => u8::MAX,
        }
    }
}

const fn shadow_part(shadow: Shadow, kind: NumericAtomKind) -> Option<f32> {
    match kind {
        NumericAtomKind::DropShadowX | NumericAtomKind::TextShadowX => Some(shadow.offset[0]),
        NumericAtomKind::DropShadowY | NumericAtomKind::TextShadowY => Some(shadow.offset[1]),
        NumericAtomKind::DropShadowBlur | NumericAtomKind::TextShadowBlur => {
            Some(shadow.blur.max(0.0))
        }
        _ => None,
    }
}

const fn bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        (value + 0.0).to_bits()
    }
}
