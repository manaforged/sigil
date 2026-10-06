use crate::style::locale::{Direction, Locale};
use crate::style::metrics::{Metrics, Radius};
use crate::style::palette::{Palette, is_dark, pick_higher_contrast};
use crate::style::scale::{
    ColorScale, ScaleRole, amber_dark, amber_light, blue_dark, blue_light, cyan_dark, cyan_light,
    green_dark, green_light, ruby_dark, ruby_light, slate_dark, slate_light,
};
use crate::style::strings::Strings;
use crate::style::variant::{FontFamilyHint, Variant};
use crate::style::{ControlSize, Dimension, LengthPercentage, Px, Rgba8};
use mix::{alpha_mix, rgba, shift};

pub(crate) mod ambient;
mod color;
mod console;
mod mix;
mod table;

pub use table::{TokenLayer, TokenTable};

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ThemeValue {
    Color(Rgba8),
    Px(f32),
    Ms(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub gray: ColorScale,
    pub accent_scale: ColorScale,
    pub success: ColorScale,
    pub warn: ColorScale,
    pub error: ColorScale,
    pub info: ColorScale,
    pub on_solid: Rgba8,

    pub accent: Rgba8,
    pub accent_hover: Rgba8,
    pub on_accent: Rgba8,
    pub surface: Rgba8,
    pub surface_hover: Rgba8,
    pub surface_pressed: Rgba8,
    pub surface_pressed_strong: Rgba8,
    pub on_surface: Rgba8,
    pub border: Rgba8,
    pub missing_token_fallback: Rgba8,
    pub tokens: TokenTable,
    pub metrics: Metrics,
    pub direction: Direction,
    pub locale: Locale,
    pub strings: Strings,
}

impl Theme {
    #[must_use]
    pub const fn scale(&self, role: ScaleRole) -> &ColorScale {
        match role {
            ScaleRole::Gray => &self.gray,
            ScaleRole::Accent => &self.accent_scale,
            ScaleRole::Success => &self.success,
            ScaleRole::Warn => &self.warn,
            ScaleRole::Error => &self.error,
            ScaleRole::Info => &self.info,
        }
    }

    #[must_use]
    pub const fn with_gray_scale(mut self, scale: ColorScale) -> Self {
        self.gray = scale;
        self.surface = scale.step(2);
        self.surface_hover = scale.step(4);
        self.surface_pressed = scale.step(5);
        self.surface_pressed_strong = scale.step(6);
        self.border = scale.step(7);
        self.on_surface = scale.step(12);
        self
    }

    #[must_use]
    pub const fn with_accent_scale(mut self, scale: ColorScale) -> Self {
        self.accent_scale = scale;
        self.accent = scale.step(9);
        self.accent_hover = scale.step(10);
        self
    }

    #[must_use]
    pub const fn with_success_scale(mut self, scale: ColorScale) -> Self {
        self.success = scale;
        self
    }

    #[must_use]
    pub const fn with_warn_scale(mut self, scale: ColorScale) -> Self {
        self.warn = scale;
        self
    }

    #[must_use]
    pub const fn with_error_scale(mut self, scale: ColorScale) -> Self {
        self.error = scale;
        self
    }

    #[must_use]
    pub const fn with_info_scale(mut self, scale: ColorScale) -> Self {
        self.info = scale;
        self
    }

    #[must_use]
    pub const fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub const fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    #[must_use]
    pub fn with_strings(mut self, strings: Strings) -> Self {
        self.strings = strings;
        self
    }

    #[must_use]
    pub const fn text_muted(&self) -> Rgba8 {
        self.gray.step(11)
    }

    #[must_use]
    pub const fn surface_subtle(&self) -> Rgba8 {
        self.gray.step(3)
    }

    #[must_use]
    pub const fn border_strong(&self) -> Rgba8 {
        self.gray.step(8)
    }
}

impl Theme {
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
        let on_solid = rgba(255, 255, 255);
        Self {
            gray,
            accent_scale,
            success,
            warn,
            error,
            info,
            on_solid,
            accent: accent_scale.step(9),
            accent_hover: accent_scale.step(10),
            on_accent: on_solid,
            surface: gray.step(2),
            surface_hover: gray.step(4),
            surface_pressed: gray.step(5),
            surface_pressed_strong: gray.step(6),
            on_surface: gray.step(12),
            border: gray.step(7),
            missing_token_fallback: rgba(255, 0, 255),
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
    #[must_use]
    pub fn from_palette(palette: Palette, variant: Variant) -> Self {
        let tokens = variant.tokens();
        let dark_surface = is_dark(palette.background);
        let hover_shift: i16 = if tokens.hover_lift {
            if dark_surface { 26 } else { -26 }
        } else {
            0
        };
        let on_accent = pick_higher_contrast(palette.primary, palette.text, palette.background);
        Self {
            gray: slate_light(),
            accent_scale: blue_light(),
            success: green_light(),
            warn: amber_light(),
            error: ruby_light(),
            info: cyan_light(),
            on_solid: rgba(255, 255, 255),
            accent: palette.primary,
            accent_hover: shift(palette.primary, hover_shift),
            on_accent,
            surface: palette.background,
            surface_hover: shift(palette.background, hover_shift),
            surface_pressed: shift(palette.background, hover_shift.saturating_mul(2)),
            surface_pressed_strong: shift(palette.background, hover_shift.saturating_mul(3)),
            on_surface: palette.text,
            border: alpha_mix(palette.text, palette.background, 0.35),
            missing_token_fallback: rgba(255, 0, 255),
            tokens: TokenTable::default(),
            metrics: Metrics::current(),
            direction: Direction::Ltr,
            locale: Locale::default(),
            strings: Strings::default(),
        }
    }

    #[must_use]
    pub fn control_width(&self, size: ControlSize) -> Dimension {
        match size {
            ControlSize::Grow => Dimension::Full,
            _ => Dimension::Px(px(self.metrics.control_width(size))),
        }
    }

    #[must_use]
    pub fn control_height(&self, size: ControlSize) -> Dimension {
        match size {
            ControlSize::Grow => Dimension::Auto,
            _ => Dimension::Px(px(self.metrics.control_height(size))),
        }
    }

    #[must_use]
    pub fn padding(&self, size: ControlSize) -> LengthPercentage {
        match size {
            ControlSize::Grow => LengthPercentage::Percent(1.0),
            _ => LengthPercentage::Px(px(self.metrics.padding(size))),
        }
    }

    #[must_use]
    pub fn gap(&self, size: ControlSize) -> LengthPercentage {
        match size {
            ControlSize::Grow => LengthPercentage::Percent(1.0),
            _ => LengthPercentage::Px(px(self.metrics.gap(size))),
        }
    }

    #[must_use]
    pub const fn radius(&self, size: ControlSize) -> LengthPercentage {
        LengthPercentage::Px(self.metrics.radius(match size {
            ControlSize::Compact => Radius::Sm,
            ControlSize::Default => Radius::Md,
            ControlSize::Wide => Radius::Lg,
            ControlSize::Grow => Radius::Full,
        }))
    }

    #[must_use]
    pub fn normal() -> Self {
        Self::from_palette(Palette::obsidian(), Variant::Normal)
    }

    #[must_use]
    pub fn terminal() -> Self {
        Self::from_palette(Palette::terminal(), Variant::Terminal)
    }
}

#[must_use]
pub const fn font_family_hint(variant: Variant) -> FontFamilyHint {
    variant.tokens().font
}

#[derive(Debug, Default, PartialEq)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn tokens(mut self, tokens: &phf::Map<&'static str, ThemeValue>) -> Self {
        self.theme = self.theme.with_theme_tokens(tokens);
        self
    }

    #[must_use]
    pub fn token(mut self, name: &str, value: ThemeValue) -> Self {
        self.theme.register_token(name, value);
        self
    }

    #[must_use]
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.theme.direction = direction;
        self
    }

    #[must_use]
    pub const fn locale(mut self, locale: Locale) -> Self {
        self.theme.locale = locale;
        self
    }

    #[must_use]
    pub fn strings(mut self, strings: Strings) -> Self {
        self.theme.strings = strings;
        self
    }

    #[must_use]
    pub fn build(self) -> Theme {
        self.theme
    }
}

fn px(value: f32) -> Px {
    let px = value
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Rounded and clamped to the i16 range before cast."
    )]
    {
        Px(px as i16)
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
