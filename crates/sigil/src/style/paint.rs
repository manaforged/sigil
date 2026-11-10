use std::time::Duration;

use crate::animation::{Easing, Keyframes, KeyframesId};
use crate::animation_runtime::presets;
use crate::kernel::ImageSource;
use crate::scene::{BlendMode, Shadow};
use crate::style::atom::{
    Amount, Backface, BorderStyle, GradientShape, MaskPart, PaintAtom, SpaceAtom, StopSlot,
};
use crate::style::engine::encode_px;
use crate::style::{
    ClipShape, ColorAtom, Conic, Elevation, FontFamily, GradientAxis, Layer, MaskImage, MaskSize,
    Radial, Radius, StyleAtom, TextDecoration, TextRole,
};

mod filters;
mod slot;
mod space;
mod units;

pub(crate) use slot::{PaintTarget, StyleSlot};
use slot::{mask, paint, push};
use units::{fraction_units, letter_spacing_units, millis, opacity_units};

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
    fn background_gradient(self, gradient: crate::style::BgGradient) -> Self {
        push(self, StyleAtom::BackgroundGradient(gradient))
    }

    /// Fills the background with a linear gradient along `axis`, like Tailwind `bg-linear-to-*`.
    #[must_use]
    fn bg_linear(self, axis: GradientAxis) -> Self {
        paint(self, PaintAtom::Gradient(GradientShape::Linear(axis)))
    }

    /// Fills the background with a radial gradient, like Tailwind `bg-radial`.
    #[must_use]
    fn bg_radial(self, radial: Radial) -> Self {
        paint(self, PaintAtom::Gradient(GradientShape::Radial(radial)))
    }

    /// Fills the background with a conic gradient, like Tailwind `bg-conic-*`.
    #[must_use]
    fn bg_conic(self, conic: impl Into<Conic>) -> Self {
        paint(
            self,
            PaintAtom::Gradient(GradientShape::Conic(conic.into())),
        )
    }

    /// Sets the first gradient color, like Tailwind `from-*`.
    #[must_use]
    fn gradient_from(self, color: impl Into<ColorAtom>) -> Self {
        paint(self, PaintAtom::Stop(StopSlot::From, color.into()))
    }

    /// Sets the middle gradient color, like Tailwind `via-*`.
    #[must_use]
    fn gradient_via(self, color: impl Into<ColorAtom>) -> Self {
        paint(self, PaintAtom::Stop(StopSlot::Via, color.into()))
    }

    /// Sets the last gradient color, like Tailwind `to-*`.
    #[must_use]
    fn gradient_to(self, color: impl Into<ColorAtom>) -> Self {
        paint(self, PaintAtom::Stop(StopSlot::To, color.into()))
    }

    /// Sets where the first color sits in percent, like Tailwind `from-<n>%`; default 0.
    #[must_use]
    fn gradient_from_position(self, percent: u8) -> Self {
        paint(self, PaintAtom::StopAt(StopSlot::From, percent.min(100)))
    }

    /// Sets where the middle color sits in percent, like Tailwind `via-<n>%`; default 50.
    #[must_use]
    fn gradient_via_position(self, percent: u8) -> Self {
        paint(self, PaintAtom::StopAt(StopSlot::Via, percent.min(100)))
    }

    /// Sets where the last color sits in percent, like Tailwind `to-<n>%`; default 100.
    #[must_use]
    fn gradient_to_position(self, percent: u8) -> Self {
        paint(self, PaintAtom::StopAt(StopSlot::To, percent.min(100)))
    }

    /// Paints the background gradient inside the glyphs of a text node, like Tailwind `bg-clip-text`.
    #[must_use]
    fn bg_clip_text(self) -> Self {
        paint(self, PaintAtom::ClipText)
    }

    /// Tiles `image` over the background at its natural size, like CSS `background-image: url(..)`.
    #[must_use]
    fn bg_image(self, image: ImageSource) -> Self {
        slot::image(self, image, |id| StyleAtom::Paint(PaintAtom::Image(id)))
    }

    /// Draws a solid border, like Tailwind `border-solid`.
    #[must_use]
    fn border_solid(self) -> Self {
        paint(self, PaintAtom::Border(BorderStyle::Solid))
    }

    /// Draws the border in dashes, like Tailwind `border-dashed`.
    #[must_use]
    fn border_dashed(self) -> Self {
        paint(self, PaintAtom::Border(BorderStyle::Dashed))
    }

    /// Draws the border in round dots, like Tailwind `border-dotted`.
    #[must_use]
    fn border_dotted(self) -> Self {
        paint(self, PaintAtom::Border(BorderStyle::Dotted))
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

    /// Clips the node and its children to `shape`, like CSS `clip-path`.
    #[must_use]
    fn clip_path(self, shape: ClipShape) -> Self {
        push(self, StyleAtom::ClipPath(shape))
    }

    /// Masks the node and its children with `image`, like CSS `mask-image`.
    #[must_use]
    fn mask_image(self, image: MaskImage) -> Self {
        slot::mask_image(self, image)
    }
    /// Sets where a gradient mask is opaque and where it is clear, in percent, like Tailwind `mask-linear-from-*` and `mask-linear-to-*`.
    #[must_use]
    fn mask_stops(self, from: u8, to: u8) -> Self {
        slot::mask_stops(self, from, to)
    }
    /// Reads the mask through its luminance instead of its alpha, like Tailwind `mask-luminance`.
    #[must_use]
    fn mask_luminance(self) -> Self {
        mask(self, MaskPart::Luminance(true))
    }
    /// Sizes the mask image against the border box, like Tailwind `mask-cover` and `mask-contain`.
    #[must_use]
    fn mask_size(self, size: MaskSize) -> Self {
        mask(self, MaskPart::Size(size))
    }
    /// Places the mask image at fractions of the free space, like Tailwind `mask-center`; default 0, 0.
    #[must_use]
    fn mask_position(self, x: f32, y: f32) -> Self {
        mask(self, MaskPart::Position([x, y].map(fraction_units)))
    }
    /// Sets whether the mask image tiles the box, like Tailwind `mask-repeat` and `mask-no-repeat`; default true.
    #[must_use]
    fn mask_repeat(self, repeat: bool) -> Self {
        mask(self, MaskPart::Repeat(repeat))
    }

    /// Sets the fill, border, corners, and shadow of a place in the visual stack.
    #[must_use]
    fn layer(self, layer: Layer) -> Self {
        layer
            .atoms()
            .iter()
            .fold(self, |target, atom| push(target, *atom))
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

    filters::filter_setters!();

    /// Sets the CSS mix-blend-mode.
    #[must_use]
    fn mix_blend(self, mode: BlendMode) -> Self {
        push(self, StyleAtom::MixBlend(mode))
    }

    #[cfg(any(test, feature = "kittest"))]
    #[doc(hidden)]
    /// Sets the transform.
    #[must_use]
    fn transform(self, transform: crate::style::Transform2D) -> Self {
        push(self, StyleAtom::Transform(transform))
    }

    space::space_setters!();

    /// Sets the size, line height, and weight of a step of the type scale.
    #[must_use]
    fn text_role(self, role: TextRole) -> Self {
        role.atoms()
            .iter()
            .fold(self, |target, atom| push(target, *atom))
    }

    /// Sets a font size off the type scale, with the reason it cannot be a `TextRole`.
    #[must_use]
    fn font_size(self, px: f32, _reason: &'static str) -> Self {
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
    fn transition_preset(self, id: crate::style::TransitionId) -> Self {
        push(self, StyleAtom::Transition(id))
    }

    /// Transitions colors, opacity, and transform over 150 ms, like Tailwind `transition`.
    #[must_use]
    fn transition(self) -> Self {
        push(self, StyleAtom::Transition(presets::TRANSITION))
    }

    /// Transitions every animatable property over 150 ms, like Tailwind `transition-all`.
    #[must_use]
    fn transition_all(self) -> Self {
        push(self, StyleAtom::Transition(presets::ALL))
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
