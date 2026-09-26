use super::Style;
use crate::style::{
    BgGradient, ColorAtom, MillisU16, OverflowKind, Px, Rgba8, StyleAtom, TransitionId,
};

impl Style {
    #[must_use]
    pub fn bg(self, color: Rgba8) -> Self {
        self.fill(ColorAtom::Rgba8(color))
    }

    #[must_use]
    pub fn fill(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::BackgroundColor(color))
    }

    #[must_use]
    pub fn background_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::BackgroundThemeRef(token))
    }

    #[must_use]
    pub fn background_gradient(self, gradient: BgGradient) -> Self {
        self.push(StyleAtom::BackgroundGradient(gradient))
    }

    #[must_use]
    pub fn text_color(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::TextColor(color))
    }

    #[must_use]
    pub fn text_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::TextThemeRef(token))
    }

    #[must_use]
    pub fn radius(self, px: i16) -> Self {
        self.push(StyleAtom::BorderRadius(Px(px)))
    }

    #[must_use]
    pub fn radius_token(self, radius: crate::style::Radius) -> Self {
        self.push(StyleAtom::Radius(radius))
    }

    #[must_use]
    #[track_caller]
    pub fn opacity(self, value: f32) -> Self {
        debug_assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "opacity must be a finite value in 0.0..=1.0, got {value}",
        );
        let clamped = value.clamp(0.0, 1.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "values are bounded to the target range before the cast"
        )]
        let encoded = (clamped * 10_000.0).round() as u16;
        self.push(StyleAtom::Opacity(encoded))
    }

    #[must_use]
    pub fn opacity_raw(self, opacity: u16) -> Self {
        self.push(StyleAtom::Opacity(opacity))
    }

    #[must_use]
    pub fn font_size(self, px: i16) -> Self {
        self.push(StyleAtom::FontSize(Px(px)))
    }

    #[must_use]
    pub fn font_weight(self, weight: u16) -> Self {
        self.push(StyleAtom::FontWeight(weight))
    }

    #[must_use]
    pub fn font_italic(self, italic: bool) -> Self {
        self.push(StyleAtom::FontItalic(italic))
    }

    #[must_use]
    pub fn font_family(self, name: &'static str) -> Self {
        self.push(StyleAtom::FontFamily(name))
    }

    #[must_use]
    pub fn line_height(self, px: i16) -> Self {
        self.push(StyleAtom::LineHeight(Px(px)))
    }

    #[must_use]
    pub fn letter_spacing_em(self, em: f32) -> Self {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "values are bounded to the target range before the cast"
        )]
        let thousandths = (em * 1000.0).round().clamp(-2000.0, 2000.0) as i16;
        self.push(StyleAtom::LetterSpacing(thousandths))
    }

    #[must_use]
    pub fn tabular_nums(self, on: bool) -> Self {
        self.push(StyleAtom::TabularNums(on))
    }

    #[must_use]
    pub fn text_decoration(self, decoration: crate::style::TextDecoration) -> Self {
        self.push(StyleAtom::TextDecoration(decoration))
    }

    #[must_use]
    pub fn line_through(self) -> Self {
        self.text_decoration(crate::style::TextDecoration::LineThrough)
    }

    #[must_use]
    pub fn underline(self) -> Self {
        self.text_decoration(crate::style::TextDecoration::Underline)
    }

    #[must_use]
    pub fn transition(self, transition: TransitionId) -> Self {
        self.push(StyleAtom::Transition(transition))
    }

    #[must_use]
    pub fn duration(self, duration: MillisU16) -> Self {
        self.push(StyleAtom::TransitionDuration(duration))
    }

    #[must_use]
    pub fn unknown_theme_ref(self, token: &'static str) -> Self {
        self.push(StyleAtom::UnknownThemeRef(token))
    }

    #[must_use]
    pub fn overflow_x(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowX(kind))
    }

    #[must_use]
    pub fn overflow_y(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowY(kind))
    }
}
