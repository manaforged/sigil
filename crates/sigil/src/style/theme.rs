use std::collections::HashMap;

use crate::style::palette::{Palette, is_dark, pick_higher_contrast};
use crate::style::scale::{
    ColorScale, ScaleRole, amber_dark, amber_light, blue_dark, blue_light, cyan_dark, cyan_light,
    green_dark, green_light, ruby_dark, ruby_light, slate_dark, slate_light,
};
use crate::style::variant::{FontFamilyHint, Variant};
use crate::style::{Dimension, LengthPercentage, Px, Rgba8, Size};

#[cfg(debug_assertions)]
const BUILT_IN_COLOR_TOKENS: [&str; 18] = [
    "accent",
    "accent-hover",
    "on-accent",
    "field",
    "surface",
    "surface-hover",
    "surface-pressed",
    "surface-pressed-strong",
    "surface-inset",
    "on-surface",
    "border",
    "border-strong",
    "scrim",
    "surface-0",
    "surface-1",
    "surface-2",
    "surface-3",
    "line",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeValue {
    Color(Rgba8),
    Px(f32),
    Ms(u16),
}

#[derive(Debug, PartialEq)]
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
    pub tokens: phf::Map<&'static str, ThemeValue>,
    pub runtime_tokens: HashMap<String, ThemeValue>,
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
    pub fn dark() -> Self {
        let gray = slate_dark();
        let accent_scale = blue_dark();
        let success = green_dark();
        let warn = amber_dark();
        let error = ruby_dark();
        let info = cyan_dark();
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
            tokens: phf::Map::new(),
            runtime_tokens: crate::style::grid::tokens(),
        }
    }

    #[must_use]
    pub fn console_dark() -> Self {
        Self {
            tokens: phf::phf_map! {
                "background" => ThemeValue::Color(Rgba8 { r: 10, g: 10, b: 10, a: 255 }),
                "foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "card" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "card-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "popover" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "popover-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "primary" => ThemeValue::Color(Rgba8 { r: 229, g: 229, b: 229, a: 255 }),
                "primary-foreground" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "secondary" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "secondary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "muted" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "muted-foreground" => ThemeValue::Color(Rgba8 { r: 161, g: 161, b: 161, a: 255 }),
                "accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "destructive" => ThemeValue::Color(Rgba8 { r: 255, g: 100, b: 103, a: 255 }),
                "destructive-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "input" => ThemeValue::Color(Rgba8 { r: 255, g: 255, b: 255, a: 38 }),
                "ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                "sidebar" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "sidebar-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-primary" => ThemeValue::Color(Rgba8 { r: 20, g: 71, b: 230, a: 255 }),
                "sidebar-primary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "sidebar-accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "sidebar-ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                "syntax-keyword" => ThemeValue::Color(Rgba8 { r: 198, g: 120, b: 221, a: 255 }),
                "syntax-string" => ThemeValue::Color(Rgba8 { r: 152, g: 195, b: 121, a: 255 }),
                "syntax-comment" => ThemeValue::Color(Rgba8 { r: 117, g: 123, b: 138, a: 255 }),
                "syntax-function" => ThemeValue::Color(Rgba8 { r: 97, g: 175, b: 239, a: 255 }),
                "syntax-type" => ThemeValue::Color(Rgba8 { r: 229, g: 192, b: 123, a: 255 }),
                "syntax-number" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-constant" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-property" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-operator" => ThemeValue::Color(Rgba8 { r: 86, g: 182, b: 194, a: 255 }),
                "syntax-punctuation" => ThemeValue::Color(Rgba8 { r: 171, g: 178, b: 191, a: 255 }),
                "syntax-variable" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-tag" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
            },
            ..Self::dark()
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        let gray = slate_light();
        let accent_scale = blue_light();
        let success = green_light();
        let warn = amber_light();
        let error = ruby_light();
        let info = cyan_light();
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
            tokens: phf::Map::new(),
            runtime_tokens: crate::style::grid::tokens(),
        }
    }
}

impl Theme {
    pub(crate) fn resolve_color(&self, token: &'static str) -> Rgba8 {
        if let Some(ThemeValue::Color(color)) = self.runtime_tokens.get(token).copied() {
            return color;
        }
        if let Some(ThemeValue::Color(color)) = self.tokens.get(token).copied() {
            return color;
        }
        if let Some(color) = self.built_in_color(token) {
            return color;
        }
        if let Some(color) = self.scale_step_color(token) {
            return color;
        }
        self.missing_color(token)
    }

    pub fn register_token(
        &mut self,
        name: impl Into<String>,
        value: ThemeValue,
    ) -> Option<ThemeValue> {
        let name = name.into();
        if let ThemeValue::Color(color) = value {
            self.sync_runtime_color(&name, color);
        }
        self.runtime_tokens.insert(name, value)
    }

    #[must_use]
    pub fn with_token(mut self, name: impl Into<String>, value: ThemeValue) -> Self {
        self.register_token(name, value);
        self
    }

    fn sync_runtime_color(&mut self, name: &str, color: Rgba8) {
        match name {
            "accent" => {
                self.accent = color;
                self.accent_scale = ramp_accent(self.surface, color);
            }
            "accent-hover" => {
                self.accent_hover = color;
                self.accent_scale.steps[9] = color;
            }
            "on-accent" => {
                self.on_accent = color;
                self.accent_scale.on_solid = color;
            }
            "surface" => self.surface = color,
            "surface-hover" => self.surface_hover = color,
            "surface-pressed" => self.surface_pressed = color,
            "surface-pressed-strong" => self.surface_pressed_strong = color,
            "on-surface" => self.on_surface = color,
            "border" => self.border = color,
            _ => {}
        }
    }

    fn built_in_color(&self, token: &str) -> Option<Rgba8> {
        match token {
            "accent" => Some(self.accent),
            "accent-hover" => Some(self.accent_hover),
            "on-accent" => Some(self.on_accent),
            "field" => Some(if is_dark(self.surface) {
                self.gray.step(3)
            } else {
                self.gray.step(1)
            }),
            "surface" | "surface-1" => Some(self.surface),
            "surface-hover" => Some(self.surface_hover),
            "surface-pressed" | "surface-2" => Some(self.surface_pressed),
            "surface-pressed-strong" | "surface-3" => Some(self.surface_pressed_strong),
            "on-surface" => Some(self.on_surface),
            "border" | "line" => Some(self.border),
            "border-strong" => Some(self.border_strong()),
            "surface-subtle" => Some(self.surface_subtle()),
            "surface-inset" | "surface-0" => Some(self.gray.step(1)),
            "on-surface-muted" => Some(self.text_muted()),
            "on-solid" => Some(self.on_solid),
            "scrim" => Some(Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 153,
            }),
            _ => None,
        }
    }

    fn scale_step_color(&self, token: &str) -> Option<Rgba8> {
        let (role_name, step_str) = token.rsplit_once('-')?;
        let role = ScaleRole::from_token(role_name)?;
        let step: u8 = step_str.parse().ok()?;
        if !(1..=12).contains(&step) {
            return None;
        }
        Some(self.scale(role).step(step))
    }

    #[cfg(debug_assertions)]
    fn missing_color(&self, token: &str) -> Rgba8 {
        let message = self.unknown_token_message(token);
        diagnostic::unknown_theme_token(&message);
    }

    #[cfg(not(debug_assertions))]
    const fn missing_color(&self, _token: &str) -> Rgba8 {
        self.missing_token_fallback
    }

    #[cfg(debug_assertions)]
    fn unknown_token_message(&self, token: &str) -> String {
        let registered = self.registered_tokens();
        let nearest = nearest_tokens(token, &registered);
        format!(
            "Unknown theme token '{token}'; registered: {}; nearest: '{}', '{}', '{}'",
            registered.join(", "),
            nearest[0],
            nearest[1],
            nearest[2],
        )
    }

    #[cfg(debug_assertions)]
    fn registered_tokens(&self) -> Vec<String> {
        let mut tokens: Vec<String> = BUILT_IN_COLOR_TOKENS.iter().map(|s| (*s).into()).collect();
        tokens.extend(self.tokens.keys().map(|s| (*s).into()));
        tokens.extend(self.runtime_tokens.keys().cloned());
        tokens
    }

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
            tokens: phf::Map::new(),
            runtime_tokens: crate::style::grid::tokens(),
        }
    }

    #[must_use]
    pub fn control_width(&self, size: Size) -> Dimension {
        self.dimension_token(
            match size {
                Size::Compact => "control-width-compact",
                Size::Default => "control-width-default",
                Size::Wide => "control-width-wide",
                Size::Grow => return Dimension::Full,
            },
            match size {
                Size::Compact => 24,
                Size::Default => 32,
                Size::Wide => 148,
                Size::Grow => unreachable!(),
            },
        )
    }

    #[must_use]
    pub fn control_height(&self, size: Size) -> Dimension {
        self.dimension_token(
            match size {
                Size::Compact => "control-height-compact",
                Size::Default => "control-height-default",
                Size::Wide => "control-height-wide",
                Size::Grow => return Dimension::Auto,
            },
            match size {
                Size::Compact => 22,
                Size::Default => 28,
                Size::Wide => 36,
                Size::Grow => unreachable!(),
            },
        )
    }

    #[must_use]
    pub fn padding(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "padding-compact",
                Size::Default => "padding-default",
                Size::Wide => "padding-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 6,
                Size::Default => 10,
                Size::Wide => 14,
                Size::Grow => unreachable!(),
            },
        )
    }

    #[must_use]
    pub fn gap(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "gap-compact",
                Size::Default => "gap-default",
                Size::Wide => "gap-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 4,
                Size::Default => 8,
                Size::Wide => 12,
                Size::Grow => unreachable!(),
            },
        )
    }

    #[must_use]
    pub fn radius(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "radius-compact",
                Size::Default => "radius-default",
                Size::Wide => "radius-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 2,
                Size::Default => 4,
                Size::Wide => 8,
                Size::Grow => unreachable!(),
            },
        )
    }

    fn dimension_token(&self, token: &'static str, fallback: i16) -> Dimension {
        Dimension::Px(self.px_token(token, fallback))
    }

    fn length_token(&self, token: &'static str, fallback: i16) -> LengthPercentage {
        LengthPercentage::Px(self.px_token(token, fallback))
    }

    fn px_token(&self, token: &'static str, fallback: i16) -> Px {
        if let Some(ThemeValue::Px(px)) = self.runtime_tokens.get(token).copied() {
            return px_from_f32(px);
        }
        match self.tokens.get(token).copied() {
            Some(ThemeValue::Px(px)) => px_from_f32(px),
            _ => Px(fallback),
        }
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

fn shift(c: Rgba8, delta: i16) -> Rgba8 {
    let nudge = |channel: u8| -> u8 {
        #[allow(
            clippy::cast_sign_loss,
            reason = "Clamped to 0..=255 before cast; no sign to lose."
        )]
        {
            i16::from(channel).saturating_add(delta).clamp(0, 255) as u8
        }
    };
    Rgba8 {
        r: nudge(c.r),
        g: nudge(c.g),
        b: nudge(c.b),
        a: c.a,
    }
}

fn ramp_accent(surface: Rgba8, solid: Rgba8) -> crate::style::scale::ColorScale {
    let mut steps = [solid; 12];
    for (step, t) in steps.iter_mut().zip(1..=9u8).take(8) {
        *step = alpha_mix(surface, solid, f32::from(t) / 9.0);
    }
    steps[8] = solid;
    steps[9] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.18,
    );
    steps[10] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.36,
    );
    steps[11] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.55,
    );
    crate::style::scale::ColorScale::from_steps(steps).with_on_solid(surface)
}

fn alpha_mix(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| -> u8 {
        let xf = f32::from(x);
        let yf = f32::from(y);
        let mixed = (1.0 - t).mul_add(xf, t * yf).round().clamp(0.0, 255.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Value clamped to 0.0..=255.0 before cast — bounded."
        )]
        {
            mixed as u8
        }
    };
    Rgba8 {
        r: blend(a.r, b.r),
        g: blend(a.g, b.g),
        b: blend(a.b, b.b),
        a: a.a,
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
    #[allow(
        clippy::missing_const_for_fn,
        reason = "Theme now owns a HashMap; const-stable in the consumer regardless."
    )]
    pub fn tokens(mut self, tokens: phf::Map<&'static str, ThemeValue>) -> Self {
        self.theme.tokens = tokens;
        self
    }

    #[must_use]
    pub fn token(mut self, name: impl Into<String>, value: ThemeValue) -> Self {
        self.theme.runtime_tokens.insert(name.into(), value);
        self
    }

    #[must_use]
    pub fn build(self) -> Theme {
        self.theme
    }
}

#[cfg(debug_assertions)]
fn nearest_tokens(token: &str, registered: &[String]) -> [String; 3] {
    let mut scored: Vec<(usize, &String)> = registered
        .iter()
        .map(|candidate| (strsim::levenshtein(token, candidate), candidate))
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let pick = |i: usize| {
        scored
            .get(i)
            .map(|(_, candidate)| (*candidate).clone())
            .unwrap_or_default()
    };
    [pick(0), pick(1), pick(2)]
}

fn px_from_f32(value: f32) -> Px {
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
const fn rgba(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

#[cfg(debug_assertions)]
mod diagnostic {
    #![allow(clippy::panic)]

    #[cold]
    #[track_caller]
    pub(super) fn unknown_theme_token(message: &str) -> ! {
        panic!("{message}");
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
