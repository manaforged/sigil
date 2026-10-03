use crate::style::engine::encode_px;
use crate::style::styled::color::{background_atom, border_color_atom, text_atom};
use crate::style::{
    BgGradient, ColorOrToken, Elevation, FontFamily, MillisU16, Px, Radius, Style, StyleAtom,
    TextDecoration, Transform2D, TransitionId,
};

mod sealed {
    use crate::style::Style;

    pub struct StyleSlot<'a>(pub(crate) &'a mut Style);

    pub trait PaintTarget: Sized {
        fn style_slot(&mut self) -> StyleSlot<'_>;
    }

    pub trait StateHost {}
}

pub(crate) use sealed::{PaintTarget, StateHost, StyleSlot};

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

pub trait Paint: PaintTarget {
    #[must_use]
    fn bg(self, color: impl Into<ColorOrToken>) -> Self {
        push(self, background_atom(color.into()))
    }

    #[must_use]
    fn background_gradient(self, gradient: BgGradient) -> Self {
        push(self, StyleAtom::BackgroundGradient(gradient))
    }

    #[must_use]
    fn text_color(self, color: impl Into<ColorOrToken>) -> Self {
        push(self, text_atom(color.into()))
    }

    #[must_use]
    fn border_color(self, color: impl Into<ColorOrToken>) -> Self {
        push(self, border_color_atom(color.into()))
    }

    #[must_use]
    fn border_w(self, px: i16) -> Self {
        push(self, StyleAtom::BorderWidth(Px(px)))
    }

    #[must_use]
    fn border_top(self, px: i16) -> Self {
        push(self, StyleAtom::BorderTop(Px(px)))
    }

    #[must_use]
    fn border_right(self, px: i16) -> Self {
        push(self, StyleAtom::BorderRight(Px(px)))
    }

    #[must_use]
    fn border_bottom(self, px: i16) -> Self {
        push(self, StyleAtom::BorderBottom(Px(px)))
    }

    #[must_use]
    fn border_left(self, px: i16) -> Self {
        push(self, StyleAtom::BorderLeft(Px(px)))
    }

    #[must_use]
    fn rounded(self, radius: Radius) -> Self {
        push(self, StyleAtom::Radius(radius))
    }

    #[must_use]
    fn shadow(self, elevation: Elevation) -> Self {
        push(self, StyleAtom::Shadow(elevation))
    }

    #[must_use]
    #[track_caller]
    fn opacity(self, value: f32) -> Self {
        push(self, StyleAtom::Opacity(opacity_units(value)))
    }

    #[must_use]
    fn transform(self, transform: Transform2D) -> Self {
        push(self, StyleAtom::Transform(transform))
    }

    #[must_use]
    fn font_size(self, px: f32) -> Self {
        push(self, StyleAtom::FontSize(encode_px(px)))
    }

    #[must_use]
    fn font_weight(self, weight: u16) -> Self {
        push(self, StyleAtom::FontWeight(weight))
    }

    #[must_use]
    fn font_italic(self, italic: bool) -> Self {
        push(self, StyleAtom::FontItalic(italic))
    }

    #[must_use]
    fn font_family(self, family: FontFamily) -> Self {
        push(self, StyleAtom::FontFamily(family.name()))
    }

    #[must_use]
    fn line_height_px(self, px: f32) -> Self {
        push(self, StyleAtom::LineHeight(encode_px(px)))
    }

    #[must_use]
    fn letter_spacing_em(self, em: f32) -> Self {
        push(self, StyleAtom::LetterSpacing(letter_spacing_units(em)))
    }

    #[must_use]
    fn tabular_nums(self, on: bool) -> Self {
        push(self, StyleAtom::TabularNums(on))
    }

    #[must_use]
    fn underline(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::Underline))
    }

    #[must_use]
    fn line_through(self) -> Self {
        push(self, StyleAtom::TextDecoration(TextDecoration::LineThrough))
    }

    #[must_use]
    fn transition(self, id: TransitionId) -> Self {
        push(self, StyleAtom::Transition(id))
    }

    #[must_use]
    fn duration(self, duration: MillisU16) -> Self {
        push(self, StyleAtom::TransitionDuration(duration))
    }
}

impl<T: PaintTarget> Paint for T {}

impl PaintTarget for Style {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(self)
    }
}
