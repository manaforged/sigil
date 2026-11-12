use crate::layout::Length;
use crate::style::locale::{Direction, Locale};
use crate::style::metrics::{Metrics, Radius};
use crate::style::palette::{Palette, is_dark, pick_higher_contrast};
use crate::style::scale::{
    ColorScale, ScaleRole, amber_dark, amber_light, blue_dark, blue_light, cyan_dark, cyan_light,
    green_dark, green_light, ruby_dark, ruby_light, slate_dark, slate_light,
};
use crate::style::srgb::{AA_TEXT, contrast};
use crate::style::strings::Strings;
#[cfg(test)]
use crate::style::variant::FontFamilyHint;
use crate::style::variant::Variant;
use crate::style::{Color, ControlSize};
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
    /// The color theme value.
    Color(Color),
    /// The px theme value.
    Px(f32),
    /// The ms theme value.
    Ms(u16),
}

/// Theme values.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The gray.
    pub gray: ColorScale,
    /// The accent scale.
    pub accent_scale: ColorScale,
    /// The success.
    pub success: ColorScale,
    /// The warn.
    pub warn: ColorScale,
    /// The error.
    pub error: ColorScale,
    /// The info.
    pub info: ColorScale,
    /// The on solid.
    pub on_solid: Color,

    /// The accent.
    pub accent: Color,
    /// The accent hover.
    pub accent_hover: Color,
    /// The on accent.
    pub on_accent: Color,
    /// The surface.
    pub surface: Color,
    /// The surface hover.
    pub surface_hover: Color,
    /// The surface pressed.
    pub surface_pressed: Color,
    /// The surface pressed strong.
    pub surface_pressed_strong: Color,
    /// The on surface.
    pub on_surface: Color,
    /// The border.
    pub border: Color,
    /// The missing token fallback.
    pub missing_token_fallback: Color,
    /// The tokens.
    pub(crate) tokens: TokenTable,
    /// The metrics.
    pub metrics: Metrics,
    /// The direction.
    pub direction: Direction,
    /// The locale.
    pub locale: Locale,
    /// The strings.
    pub strings: Strings,
}

impl Theme {
    /// The 12-step color scale of `role`.
    #[must_use]
    pub const fn scale(&self, role: ScaleRole) -> &ColorScale {
        match role {
            ScaleRole::Gray => &self.gray,
            ScaleRole::Accent => &self.accent_scale,
            ScaleRole::Success => &self.success,
            ScaleRole::Warning => &self.warn,
            ScaleRole::Error => &self.error,
            ScaleRole::Info => &self.info,
        }
    }

    #[cfg(feature = "kittest")]
    /// Sets the gray scale and the surface, border and text colors drawn from it.
    #[must_use]
    #[doc(hidden)]
    pub const fn gray(mut self, scale: ColorScale) -> Self {
        self.gray = scale;
        self.surface = scale.step(2);
        self.surface_hover = scale.step(4);
        self.surface_pressed = scale.step(5);
        self.surface_pressed_strong = scale.step(6);
        self.border = scale.step(7);
        self.on_surface = scale.step(12);
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the accent scale, and the accent and accent hover colors from its steps 9 and 10.
    #[must_use]
    #[doc(hidden)]
    pub const fn accent_scale(mut self, scale: ColorScale) -> Self {
        self.accent_scale = scale;
        self.accent = scale.step(9);
        self.accent_hover = scale.step(10);
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the color scale of the success role.
    #[must_use]
    #[doc(hidden)]
    pub const fn success(mut self, scale: ColorScale) -> Self {
        self.success = scale;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the color scale of the warning role.
    #[must_use]
    #[doc(hidden)]
    pub const fn warn(mut self, scale: ColorScale) -> Self {
        self.warn = scale;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the color scale of the error role.
    #[must_use]
    #[doc(hidden)]
    pub const fn error(mut self, scale: ColorScale) -> Self {
        self.error = scale;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the color scale of the info role.
    #[must_use]
    #[doc(hidden)]
    pub const fn info(mut self, scale: ColorScale) -> Self {
        self.info = scale;
        self
    }

    /// Sets the layout direction: left to right or right to left.
    #[must_use]
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the number and date formats of the theme.
    #[must_use]
    #[doc(hidden)]
    pub const fn locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    #[cfg(feature = "kittest")]
    /// Sets the string table of the theme.
    #[must_use]
    #[doc(hidden)]
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
