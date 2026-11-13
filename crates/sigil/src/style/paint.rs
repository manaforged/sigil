use crate::animation::{Keyframes, KeyframesId, Transition};
use crate::kernel::ImageSource;
use crate::scene::{BlendMode, Shadow};
use crate::style::atom::{Amount, Backface, BorderStyle, MaskPart, PaintAtom, SpaceAtom};
use crate::style::engine::encode_px;
use crate::style::{
    Background, ClipShape, ColorValue, Elevation, FontFamily, Layer, MaskImage, MaskSize, Radius,
    StateStyleBuilder, StyleAtom, TextDecoration, TextRole,
};

mod filters;
mod slot;
mod space;
mod units;

use slot::{mask, paint, push};
use units::{fraction_units, letter_spacing_units, opacity_units};

/// Typed paint on nodes, text, widget builders, and state closures.
pub trait Paint: Sized {
    /// Returns the mutable style builder for this target.
    fn paint_mut(&mut self) -> &mut StateStyleBuilder;

    /// Fills the background with a color or a gradient.
    #[must_use]
    fn background(self, background: impl Into<Background>) -> Self {
        background.into().atoms().into_iter().fold(self, push)
    }

    /// Paints the background gradient inside the glyphs of a text node, like Tailwind `bg-clip-text`.
    #[must_use]
    fn background_clip_text(self) -> Self {
        paint(self, PaintAtom::ClipText)
    }

    /// Tiles `image` over the background at its natural size, like CSS `background-image: url(..)`.
    #[must_use]
    fn background_image(self, image: ImageSource) -> Self {
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

    /// Sets the color of the text.
    #[must_use]
    fn text_color(self, color: impl Into<ColorValue>) -> Self {
        push(self, StyleAtom::TextColor(color.into()))
    }

    /// Sets the color of the border.
    #[must_use]
    fn border_color(self, color: impl Into<ColorValue>) -> Self {
        push(self, StyleAtom::BorderColor(color.into()))
    }

    /// Sets the width of the border on all four sides in pixels.
    #[must_use]
    fn border_width(self, px: i16) -> Self {
        push(self, StyleAtom::BorderWidth(px))
    }

    /// Sets the width of the top border in pixels.
    #[must_use]
    fn border_top(self, px: i16) -> Self {
        push(self, StyleAtom::BorderTop(px))
    }

    /// Sets the width of the right border in pixels.
    #[must_use]
    fn border_right(self, px: i16) -> Self {
        push(self, StyleAtom::BorderRight(px))
    }

    /// Sets the width of the bottom border in pixels.
    #[must_use]
    fn border_bottom(self, px: i16) -> Self {
        push(self, StyleAtom::BorderBottom(px))
    }

    /// Sets the width of the left border in pixels.
    #[must_use]
    fn border_left(self, px: i16) -> Self {
        push(self, StyleAtom::BorderLeft(px))
    }

    /// Sets the corner radius to a step of the radius scale.
    #[must_use]
    fn radius(self, radius: Radius) -> Self {
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

    /// Sets the shadow to a step of the elevation scale.
    #[must_use]
    fn shadow(self, elevation: Elevation) -> Self {
        push(self, StyleAtom::Shadow(elevation))
    }

    /// Sets the opacity from 0.0 (clear) to 1.0 (opaque).
    #[must_use]
    #[track_caller]
    fn opacity(self, value: f32) -> Self {
        push(self, StyleAtom::Opacity(opacity_units(value)))
    }

    filters::filter_setters!();

    /// Sets the CSS mix-blend-mode.
    #[must_use]
    fn blend(self, mode: BlendMode) -> Self {
        push(self, StyleAtom::MixBlend(mode))
    }

    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    /// Sets the 2D transform of the node.
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

    /// Sets the weight of the font as a number.
    #[must_use]
    fn font_weight(self, weight: u16) -> Self {
        push(self, StyleAtom::FontWeight(weight))
    }

    /// Sets whether the text is italic.
    #[must_use]
    fn font_italic(self, italic: bool) -> Self {
        push(self, StyleAtom::FontItalic(italic))
    }

    /// Sets the font family of the text.
    #[must_use]
    fn font_family(self, family: FontFamily) -> Self {
        push(self, StyleAtom::FontFamily(family.name()))
    }

    /// Sets the line height in pixels.
    #[must_use]
    fn line_height(self, px: f32) -> Self {
        push(self, StyleAtom::LineHeight(encode_px(px).0))
    }

    /// Sets the letter spacing in ems.
    #[must_use]
    fn letter_spacing(self, em: f32) -> Self {
        push(self, StyleAtom::LetterSpacing(letter_spacing_units(em)))
    }

    /// Sets whether the text uses tabular numerals.
    #[must_use]
    fn tabular_nums(self, on: bool) -> Self {
        push(self, StyleAtom::TabularNums(on))
    }

    /// Underlines the text.
    #[must_use]
    fn underline(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::Underline))
    }

    /// Draws a line through the text.
    #[must_use]
    fn line_through(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::LineThrough))
    }

    /// Animates the properties that `transition` names when their values change, like CSS `transition`.
    #[must_use]
    fn transition(self, transition: impl Into<Transition>) -> Self {
        transition.into().atoms().into_iter().fold(self, push)
    }

    /// Plays a keyframe animation, like Tailwind `animate-*`.
    #[must_use]
    fn animation(self, keyframes: Keyframes) -> Self {
        push(self, StyleAtom::Animation(KeyframesId::new(keyframes)))
    }
}
