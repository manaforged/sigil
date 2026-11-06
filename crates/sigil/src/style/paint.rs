#[cfg(any(test, feature = "kittest"))]
use crate::style::BgGradient;
#[cfg(any(test, feature = "kittest"))]
use crate::style::Transform2D;
#[cfg(any(test, feature = "kittest"))]
use crate::style::TransitionId;
use std::time::Duration;

use crate::animation::{Easing, Keyframes, KeyframesId};
use crate::animation_runtime::presets;
use crate::style::MillisU16;
use crate::style::engine::encode_px;
use crate::style::{ColorAtom, Elevation, FontFamily, Radius, Style, StyleAtom, TextDecoration};

mod sealed {
    use crate::style::Style;

    pub struct StyleSlot<'a>(pub(crate) &'a mut Style);

    pub trait PaintTarget: Sized {
        fn style_slot(&mut self) -> StyleSlot<'_>;
    }
}

pub(crate) use sealed::{PaintTarget, StyleSlot};

const OPACITY_SCALE: f32 = 10_000.0;
const LETTER_SPACING_SCALE: f32 = 1_000.0;
const LETTER_SPACING_LIMIT: f32 = 2_000.0;

fn push<T: PaintTarget>(mut target: T, atom: StyleAtom) -> T {
    target.style_slot().0.push_atom(atom);
    target
}

#[track_caller]
fn opacity_units(value: f32) -> u16 {
    debug_assert!(
        value.is_finite() && (0.0..=1.0).contains(&value),
        "opacity must be a finite value in 0.0..=1.0, got {value}",
    );
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let encoded = (value.clamp(0.0, 1.0) * OPACITY_SCALE).round() as u16;
    encoded
}

fn millis(duration: Duration) -> MillisU16 {
    MillisU16(u16::try_from(duration.as_millis()).unwrap_or(u16::MAX))
}

fn letter_spacing_units(em: f32) -> i16 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    let thousandths = (em * LETTER_SPACING_SCALE)
        .round()
        .clamp(-LETTER_SPACING_LIMIT, LETTER_SPACING_LIMIT) as i16;
    thousandths
}

/// Typed paint on nodes, text, widget builders, and state closures.
pub trait Paint: PaintTarget {
    /// Sets the bg.
    #[must_use]
    fn bg(self, color: impl Into<ColorAtom>) -> Self {
        push(self, StyleAtom::BackgroundColor(color.into()))
    }

    #[cfg(any(test, feature = "kittest"))]
    #[doc(hidden)]
    /// Sets the background gradient.
    #[must_use]
    fn background_gradient(self, gradient: BgGradient) -> Self {
        push(self, StyleAtom::BackgroundGradient(gradient))
    }

    /// Sets the text color.
    #[must_use]
    fn text_color(self, color: impl Into<ColorAtom>) -> Self {
        push(self, StyleAtom::TextColor(color.into()))
    }

    /// Sets the border color.
    #[must_use]
    fn border_color(self, color: impl Into<ColorAtom>) -> Self {
        push(self, StyleAtom::BorderColor(color.into()))
    }

    /// Sets the border w.
    #[must_use]
    fn border_w(self, px: i16) -> Self {
        push(self, StyleAtom::BorderWidth(px))
    }

    /// Sets the border top.
    #[must_use]
    fn border_top(self, px: i16) -> Self {
        push(self, StyleAtom::BorderTop(px))
    }

    /// Sets the border right.
    #[must_use]
    fn border_right(self, px: i16) -> Self {
        push(self, StyleAtom::BorderRight(px))
    }

    /// Sets the border bottom.
    #[must_use]
    fn border_bottom(self, px: i16) -> Self {
        push(self, StyleAtom::BorderBottom(px))
    }

    /// Sets the border left.
    #[must_use]
    fn border_left(self, px: i16) -> Self {
        push(self, StyleAtom::BorderLeft(px))
    }

    /// Sets the rounded.
    #[must_use]
    fn rounded(self, radius: Radius) -> Self {
        push(self, StyleAtom::Radius(radius))
    }

    /// Sets the shadow.
    #[must_use]
    fn shadow(self, elevation: Elevation) -> Self {
        push(self, StyleAtom::Shadow(elevation))
    }

    /// Sets the opacity.
    #[must_use]
    #[track_caller]
    fn opacity(self, value: f32) -> Self {
        push(self, StyleAtom::Opacity(opacity_units(value)))
    }

    #[cfg(any(test, feature = "kittest"))]
    #[doc(hidden)]
    /// Sets the transform.
    #[must_use]
    fn transform(self, transform: Transform2D) -> Self {
        push(self, StyleAtom::Transform(transform))
    }

    /// Sets the font size.
    #[must_use]
    fn font_size(self, px: f32) -> Self {
        push(self, StyleAtom::FontSize(encode_px(px).0))
    }

    /// Sets the font weight.
    #[must_use]
    fn font_weight(self, weight: u16) -> Self {
        push(self, StyleAtom::FontWeight(weight))
    }

    /// Sets the font italic.
    #[must_use]
    fn font_italic(self, italic: bool) -> Self {
        push(self, StyleAtom::FontItalic(italic))
    }

    /// Sets the font family.
    #[must_use]
    fn font_family(self, family: FontFamily) -> Self {
        push(self, StyleAtom::FontFamily(family.name()))
    }

    /// Sets the line height px.
    #[must_use]
    fn line_height_px(self, px: f32) -> Self {
        push(self, StyleAtom::LineHeight(encode_px(px).0))
    }

    /// Sets the letter spacing em.
    #[must_use]
    fn letter_spacing_em(self, em: f32) -> Self {
        push(self, StyleAtom::LetterSpacing(letter_spacing_units(em)))
    }

    /// Sets the tabular nums.
    #[must_use]
    fn tabular_nums(self, on: bool) -> Self {
        push(self, StyleAtom::TabularNums(on))
    }

    /// Applies the underline option.
    #[must_use]
    fn underline(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::Underline))
    }

    /// Applies the line through option.
    #[must_use]
    fn line_through(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::LineThrough))
    }

    #[cfg(any(test, feature = "kittest"))]
    #[doc(hidden)]
    /// Sets the transition preset.
    #[must_use]
    fn transition_preset(self, id: TransitionId) -> Self {
        push(self, StyleAtom::Transition(id))
    }

    /// Transitions colors, opacity, and transform over 150 ms, like Tailwind `transition`.
    #[must_use]
    fn transition(self) -> Self {
        push(self, StyleAtom::Transition(presets::TRANSITION))
    }

    /// Transitions background, text, and border colors, like Tailwind `transition-colors`.
    #[must_use]
    fn transition_colors(self) -> Self {
        push(self, StyleAtom::Transition(presets::COLORS))
    }

    /// Transitions opacity, like Tailwind `transition-opacity`.
    #[must_use]
    fn transition_opacity(self) -> Self {
        push(self, StyleAtom::Transition(presets::OPACITY))
    }

    /// Transitions translate, scale, and rotate, like Tailwind `transition-transform`.
    #[must_use]
    fn transition_transform(self) -> Self {
        push(self, StyleAtom::Transition(presets::TRANSFORM))
    }

    /// Sets the transition duration, like Tailwind `duration-*`.
    #[must_use]
    fn duration(self, duration: Duration) -> Self {
        push(self, StyleAtom::TransitionDuration(millis(duration)))
    }

    /// Sets the transition easing, like Tailwind `ease-*`.
    #[must_use]
    fn ease(self, easing: Easing) -> Self {
        push(self, StyleAtom::TransitionEase(easing))
    }

    /// Holds the start value before a transition moves, like Tailwind `delay-*`.
    #[must_use]
    fn delay(self, delay: Duration) -> Self {
        push(self, StyleAtom::TransitionDelay(millis(delay)))
    }

    /// Plays a keyframe animation, like Tailwind `animate-*`.
    #[must_use]
    fn animate(self, keyframes: Keyframes) -> Self {
        push(self, StyleAtom::Animation(KeyframesId::new(keyframes)))
    }
}

impl<T: PaintTarget> Paint for T {}

impl PaintTarget for Style {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(self)
    }
}
