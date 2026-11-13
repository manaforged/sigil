use crate::layout::Length;
use crate::style::locale::{Direction, Locale};
use crate::style::metrics::{Metrics, Radius};
use crate::style::palette::{Palette, is_dark, pick_higher_contrast};
use crate::style::scale::{
    ColorScale, amber_dark, amber_light, blue_dark, blue_light, cyan_dark, cyan_light, green_dark,
    green_light, ruby_dark, ruby_light, slate_dark, slate_light,
};
use crate::style::srgb::{AA_TEXT, contrast};
use crate::style::strings::Strings;
#[cfg(test)]
use crate::style::variant::FontFamilyHint;
use crate::style::variant::Variant;
use crate::style::{Color, ControlSize, Tone};
use mix::{alpha_mix, shift};

const HOVER_DARKEN: i16 = -14;

pub(crate) mod ambient;
mod color;
mod console;
mod mix;
mod table;

pub(crate) use ambient::fallback_theme;
pub(crate) use table::TokenTable;

/// One theme token value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ThemeValue {
    /// A color.
    Color(Color),
    /// A length in pixels.
    Px(f32),
    /// A duration in milliseconds.
    Ms(u16),
}

/// Theme values.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub(crate) gray: ColorScale,
    pub(crate) accent_scale: ColorScale,
    pub(crate) success: ColorScale,
    pub(crate) warn: ColorScale,
    pub(crate) error: ColorScale,
    pub(crate) info: ColorScale,
    pub(crate) on_solid: Color,

    pub(crate) accent: Color,
    pub(crate) accent_hover: Color,
    pub(crate) on_accent: Color,
    pub(crate) surface: Color,
    pub(crate) surface_hover: Color,
    pub(crate) surface_pressed: Color,
    pub(crate) surface_pressed_strong: Color,
    pub(crate) on_surface: Color,
    pub(crate) border: Color,
    pub(crate) missing_token_fallback: Color,
    pub(crate) tokens: TokenTable,
    pub(crate) metrics: Metrics,
    pub(crate) direction: Direction,
    pub(crate) locale: Locale,
    pub(crate) strings: Strings,
}

impl Theme {
    /// The 12-step color scale of `tone`.
    #[must_use]
    pub const fn scale(&self, tone: Tone) -> &ColorScale {
        match tone {
            Tone::Neutral => &self.gray,
            Tone::Primary => &self.accent_scale,
            Tone::Success => &self.success,
            Tone::Warning => &self.warn,
            Tone::Error => &self.error,
            Tone::Info => &self.info,
        }
    }

    /// Sets the 12-step color scale of `tone`, updating derived surface or accent colors.
    #[must_use]
    pub fn tone(mut self, tone: Tone, scale: ColorScale) -> Self {
        match tone {
            Tone::Neutral => {
                self.gray = scale;
                self.surface = scale.step(2);
                self.surface_hover = scale.step(4);
                self.surface_pressed = scale.step(5);
                self.surface_pressed_strong = scale.step(6);
                self.border = scale.step(7);
                self.on_surface = scale.step(12);
            }
            Tone::Primary => {
                self.accent_scale = scale;
                self.accent = scale.step(9);
                self.accent_hover = readable_hover(scale, self.on_solid);
            }
            Tone::Success => self.success = scale,
            Tone::Warning => self.warn = scale,
            Tone::Error => self.error = scale,
            Tone::Info => self.info = scale,
        }
        self
    }

    /// Sets the layout direction: left to right or right to left.
    #[must_use]
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the number and date formats of the theme.
    #[must_use]
    pub const fn locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    /// Sets the string table of the theme.
    #[must_use]
    pub fn strings(mut self, strings: Strings) -> Self {
        self.strings = strings;
        self
    }
}

fn readable_hover(accent: ColorScale, on: Color) -> Color {
    let lighter = accent.step(10);
    if contrast(lighter, on) >= AA_TEXT {
        lighter
    } else {
        shift(accent.step(9), HOVER_DARKEN)
    }
}

impl Theme {
    /// Creates the light theme from the built-in light color scales.
    #[must_use]
    pub fn light() -> Self {
        Self::from_scales(&[
            slate_light(),
            blue_light(),
            green_light(),
            amber_light(),
            ruby_light(),
            cyan_light(),
        ])
    }

    /// Creates the dark theme from the built-in dark color scales.
    #[must_use]
    pub fn dark() -> Self {
        Self::from_scales(&[
            slate_dark(),
            blue_dark(),
            green_dark(),
            amber_dark(),
            ruby_dark(),
            cyan_dark(),
        ])
    }

    fn from_scales(scales: &[ColorScale; 6]) -> Self {
        let [gray, accent_scale, success, warn, error, info] = *scales;
        let on_solid = Color::rgb(255, 255, 255);
        Self {
            gray,
            accent_scale,
            success,
            warn,
            error,
            info,
            on_solid,
            accent: accent_scale.step(9),
            accent_hover: readable_hover(accent_scale, on_solid),
            on_accent: on_solid,
            surface: gray.step(2),
            surface_hover: gray.step(4),
            surface_pressed: gray.step(5),
            surface_pressed_strong: gray.step(6),
            on_surface: gray.step(12),
            border: gray.step(7),
            missing_token_fallback: Color::rgb(255, 0, 255),
            tokens: TokenTable::default(),
            metrics: Metrics::current(),
            direction: Direction::Ltr,
            locale: Locale::default(),
            strings: Strings::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// Creates a theme from palette.
    #[must_use]
    pub(crate) fn from_palette(palette: Palette, variant: Variant) -> Self {
        let tokens = variant.tokens();
        let dark_surface = is_dark(palette.background);
        let hover_shift: i16 = if tokens.hover_lift {
            if dark_surface { 26 } else { -26 }
        } else {
            0
        };
        let on_accent = pick_higher_contrast(palette.primary, palette.text, palette.background);
        Self {
            accent: palette.primary,
            accent_hover: shift(palette.primary, hover_shift),
            on_accent,
            surface: palette.background,
            surface_hover: shift(palette.background, hover_shift),
            surface_pressed: shift(palette.background, hover_shift.saturating_mul(2)),
            surface_pressed_strong: shift(palette.background, hover_shift.saturating_mul(3)),
            on_surface: palette.text,
            border: alpha_mix(palette.text, palette.background, 0.35),
            ..Self::light()
        }
    }

    /// The width of a control of `size`, as a pixel length.
    #[must_use]
    pub const fn control_width(&self, size: ControlSize) -> Length {
        Length::Px(self.metrics.control_width(size))
    }

    /// The height of a control of `size`, as a pixel length.
    #[must_use]
    pub fn control_height(&self, size: ControlSize) -> Length {
        Length::Px(self.metrics.control_height(size))
    }

    /// The padding of a control of `size`, as a pixel length.
    #[must_use]
    pub fn control_padding(&self, size: ControlSize) -> Length {
        Length::Px(self.metrics.control_padding(size))
    }

    /// The gap of a control of `size`, as a pixel length.
    #[must_use]
    pub fn control_gap(&self, size: ControlSize) -> Length {
        Length::Px(self.metrics.control_gap(size))
    }

    /// The corner radius of a control of `size`, as a pixel length.
    #[must_use]
    pub const fn radius(&self, size: ControlSize) -> Length {
        Length::Px(self.metrics.radius(match size {
            ControlSize::Compact => Radius::Sm,
            ControlSize::Default => Radius::Md,
            ControlSize::Wide => Radius::Lg,
        }))
    }

    /// Creates a theme from the obsidian palette with the normal variant.
    #[must_use]
    pub fn normal() -> Self {
        Self::from_palette(Palette::obsidian(), Variant::Normal)
    }

    /// Creates a theme from the terminal palette with the terminal variant.
    #[must_use]
    pub fn terminal() -> Self {
        Self::from_palette(Palette::terminal(), Variant::Terminal)
    }
}

#[cfg(test)]
const fn font_family_hint(variant: Variant) -> FontFamilyHint {
    variant.tokens().font
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
