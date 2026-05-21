//! Theme tokens and palette values.

use crate::palette::Palette;
use crate::variant::{FontFamilyHint, Variant};
use crate::{Dimension, LengthPercentage, Px, Rgba8, Size};

#[cfg(debug_assertions)]
const BUILT_IN_COLOR_TOKENS: [&str; 9] = [
    "accent",
    "accent-hover",
    "on-accent",
    "surface",
    "surface-hover",
    "surface-pressed",
    "surface-pressed-strong",
    "on-surface",
    "border",
];

/// A value stored in the user-extensible theme token map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeValue {
    /// A concrete colour.
    Color(Rgba8),
    /// A logical-pixel value.
    Px(f32),
    /// A millisecond duration.
    Ms(u16),
}

/// Runtime theme snapshot used to resolve style atoms.
#[derive(Debug, PartialEq)]
pub struct Theme {
    /// Primary action/accent colour.
    pub accent: Rgba8,
    /// Hover state for primary action/accent surfaces.
    pub accent_hover: Rgba8,
    /// Text/icon colour on accent surfaces.
    pub on_accent: Rgba8,
    /// Default panel/surface colour.
    pub surface: Rgba8,
    /// Hover state for default surfaces.
    pub surface_hover: Rgba8,
    /// Pressed/active state for default surfaces. Visibly deeper than
    /// [`surface_hover`] so a 16px swatch can carry the delta on its
    /// own without needing a border shift.
    pub surface_pressed: Rgba8,
    /// Deepest pressed-state tint for tiny indicators (16-20 px) where
    /// the hover→pressed step on [`surface_pressed`] alone fell below
    /// the same-hue JND (~6 sRGB units measured at the 75% opacity
    /// blend the previous design used). Roughly twice the linear-space
    /// step from `surface` that `surface_pressed` carries — clears the
    /// JND at 16 px swatch scale without needing a border cue.
    pub surface_pressed_strong: Rgba8,
    /// Text/icon colour on default surfaces.
    pub on_surface: Rgba8,
    /// Default border colour.
    pub border: Rgba8,
    /// Release-mode fallback for missing theme tokens.
    pub missing_token_fallback: Rgba8,
    /// User-defined tokens.
    pub tokens: phf::Map<&'static str, ThemeValue>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: rgba(51, 112, 255), // Clear accent for proof apps and early widgets.
            accent_hover: rgba(35, 93, 230), // Slightly deeper accent for hover feedback.
            on_accent: rgba(255, 255, 255), // White keeps accent controls readable.
            surface: rgba(248, 250, 252), // Near-white default pane surface.
            // Hover sits ~26 lightness units darker than `surface` so the
            // post-sRGB-gamma delta clears the same-hue JND (~7.65) on
            // small swatches (16-22px). Previous `(226, 232, 240)` gave a
            // ~8 sRGB delta — borderline-to-below the JND, sub-perceptual.
            surface_hover: rgba(200, 210, 222), // Clearly perceptible hover lift.
            // Pressed sits ~52 units darker — twice the hover delta, so the
            // hover→pressed step itself is also above JND. Previous
            // `(192, 202, 217)` collided with hover at small widget scale.
            surface_pressed: rgba(160, 174, 192), // Markedly darker press tint.
            // Strong-pressed sits ~26 units below `surface_pressed`
            // (matching the hover→pressed step). Targeted at 16-20 px
            // indicators where the previous 75 %-opacity blend over
            // `surface_pressed` produced only a 6 sRGB delta vs hover.
            surface_pressed_strong: rgba(130, 144, 162), // Deepest tint for tiny indicators.
            on_surface: rgba(15, 23, 42),                // Dark text for light surfaces.
            border: rgba(148, 163, 184),                 // Subtle cool border.
            missing_token_fallback: rgba(255, 0, 255),   // Debug magenta for misses.
            tokens: phf::Map::new(),
        }
    }
}

impl Theme {
    pub(crate) fn resolve_color(&self, token: &'static str) -> Rgba8 {
        if let Some(color) = self.built_in_color(token) {
            return color;
        }
        if let Some(ThemeValue::Color(color)) = self.tokens.get(token).copied() {
            return color;
        }
        self.missing_color(token)
    }

    fn built_in_color(&self, token: &str) -> Option<Rgba8> {
        match token {
            "accent" => Some(self.accent),
            "accent-hover" => Some(self.accent_hover),
            "on-accent" => Some(self.on_accent),
            "surface" => Some(self.surface),
            "surface-hover" => Some(self.surface_hover),
            "surface-pressed" => Some(self.surface_pressed),
            "surface-pressed-strong" => Some(self.surface_pressed_strong),
            "on-surface" => Some(self.on_surface),
            "border" => Some(self.border),
            _ => None,
        }
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
    fn registered_tokens(&self) -> Vec<&'static str> {
        let mut tokens = BUILT_IN_COLOR_TOKENS.to_vec();
        tokens.extend(self.tokens.keys().copied());
        tokens
    }

    /// Compose a [`Theme`] from a six-role [`Palette`] and a shape
    /// [`Variant`]. The internal seven-token colour set (accent /
    /// surface / border family) is derived from the palette; the
    /// variant's hover behaviour decides whether `surface_hover` and
    /// `accent_hover` shift versus their base.
    ///
    /// This is the foundational constructor every named theme
    /// (Obsidian, Mythril, Aetherlight, …) delegates to.
    #[must_use]
    pub fn from_palette(palette: Palette, variant: Variant) -> Self {
        let tokens = variant.tokens();
        let dark_surface = is_dark(palette.background);
        // Hover shifts by ~26 lightness units (was 18). The 18-unit
        // value gave only ~8 sRGB delta after gamma encoding —
        // borderline-to-below the same-hue JND (~7.65) and sub-
        // perceptual on a 16-22px widget swatch. 26 lifts the post-
        // gamma delta clear of the JND across calibration variance.
        // Pressed doubles this (saturating to 52) so the hover→pressed
        // step is itself perceptible.
        let hover_shift: i16 = if tokens.hover_lift {
            if dark_surface { 26 } else { -26 }
        } else {
            // Variant suppresses hover lift (e.g. Terminal): surfaces
            // stay flat regardless of pointer state.
            0
        };
        // Text-on-accent: pick whichever of the palette's foreground or
        // background colour better contrasts with the accent. The prior
        // `on_accent: palette.background` was correct only by accident
        // for palettes where the accent and the background sat on
        // opposite ends of the brightness axis; for `Spellforge` with a
        // dark accent on a dark palette it produced unreadable dark-on-
        // dark text.
        let on_accent = pick_higher_contrast(palette.primary, palette.text, palette.background);
        Self {
            accent: palette.primary,
            accent_hover: shift(palette.primary, hover_shift),
            on_accent,
            surface: palette.background,
            surface_hover: shift(palette.background, hover_shift),
            // Pressed shifts twice as far as hover — a Terminal-flat
            // variant collapses both to the base alongside hover.
            surface_pressed: shift(palette.background, hover_shift.saturating_mul(2)),
            // Strong-pressed shifts a further `hover_shift` past pressed
            // (3× hover from surface). Terminal-flat variants collapse
            // alongside hover/pressed. Targeted at 16-20 px indicators.
            surface_pressed_strong: shift(palette.background, hover_shift.saturating_mul(3)),
            on_surface: palette.text,
            border: alpha_mix(palette.text, palette.background, 0.35),
            missing_token_fallback: rgba(255, 0, 255),
            tokens: phf::Map::new(),
        }
    }

    /// Resolve the preferred control width for a semantic size preset.
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

    /// Resolve the preferred control height for a semantic size preset.
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
                Size::Compact => 24,
                Size::Default => 32,
                Size::Wide => 44,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve uniform padding for a semantic size preset.
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
                Size::Compact => 8,
                Size::Default => 12,
                Size::Wide => 16,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve uniform gap for a semantic size preset.
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

    /// Resolve border radius for a semantic size preset.
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
        match self.tokens.get(token).copied() {
            Some(ThemeValue::Px(px)) => px_from_f32(px),
            _ => Px(fallback),
        }
    }
    /// The non-terminal regression baseline — Obsidian palette under
    /// the Normal shape variant. The default Sigil look.
    #[must_use]
    pub fn normal() -> Self {
        Self::from_palette(Palette::obsidian(), Variant::Normal)
    }

    /// The terminal regression baseline — Terminal palette under the
    /// Terminal shape variant. Green-on-black monospace stress-test.
    #[must_use]
    pub fn terminal() -> Self {
        Self::from_palette(Palette::terminal(), Variant::Terminal)
    }
}

const fn is_dark(c: Rgba8) -> bool {
    // Cheap perceived-brightness gate — bright pixels return false.
    // 30R + 59G + 11B / 100 < 128 ⇒ dark.
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    (r * 30 + g * 59 + b * 11) < 12_800
}

/// Returns the candidate (`a` or `b`) whose perceived-brightness sits
/// further from `against` on the gamma-encoded BT.601 axis.
///
/// Used by [`Theme::from_palette`] to pick `on_accent` text colour:
/// against a dark accent the brighter of the two palette colours wins,
/// and vice versa. Gamma-uncorrected — adequate for the binary "which
/// is more contrasty" decision but **not** a WCAG contrast ratio.
const fn pick_higher_contrast(against: Rgba8, a: Rgba8, b: Rgba8) -> Rgba8 {
    let target = perceived_brightness_u32(against);
    let da = perceived_brightness_u32(a).abs_diff(target);
    let db = perceived_brightness_u32(b).abs_diff(target);
    if da >= db { a } else { b }
}

const fn perceived_brightness_u32(c: Rgba8) -> u32 {
    // 30R + 59G + 11B (unscaled). Range 0..=25500 for any u8 input.
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    r * 30 + g * 59 + b * 11
}

/// Shift a colour's per-channel brightness by `delta`, clamping to the
/// 0..=255 range. Used for hover lifts.
fn shift(c: Rgba8, delta: i16) -> Rgba8 {
    let nudge = |channel: u8| -> u8 {
        // `clamp(0, 255)` constrains the value to `0..=255`, so the
        // downcast is exact — documents the invariant rather than
        // masking a real lossy cast.
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

/// Linear-RGB lerp between `a` and `b` using `t ∈ [0, 1]` as the
/// weight on `b`. Used to derive subtle border colours from a
/// foreground+background pair.
fn alpha_mix(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| -> u8 {
        let xf = f32::from(x);
        let yf = f32::from(y);
        let mixed = (1.0 - t).mul_add(xf, t * yf).round().clamp(0.0, 255.0);
        // `mixed` is rounded then clamped to `0.0..=255.0` — the cast
        // to `u8` is exact for every representable value. Document
        // the proof rather than mask a real lossy cast.
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

/// Convenience access to the [`FontFamilyHint`] a [`Theme`] inherited
/// from its [`Variant`]. Renderers consult this when picking a face.
#[must_use]
pub const fn font_family_hint(variant: Variant) -> FontFamilyHint {
    variant.tokens().font
}

/// Builder for constructing a theme with user tokens.
#[derive(Debug, Default, PartialEq)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    /// Start from the default Sigil theme.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the user token map.
    #[must_use]
    pub const fn tokens(mut self, tokens: phf::Map<&'static str, ThemeValue>) -> Self {
        self.theme.tokens = tokens;
        self
    }

    /// Build the theme.
    #[must_use]
    pub const fn build(self) -> Theme {
        self.theme
    }
}

#[cfg(debug_assertions)]
fn nearest_tokens(token: &str, registered: &[&'static str]) -> [&'static str; 3] {
    let mut scored = registered
        .iter()
        .copied()
        .map(|candidate| (strsim::levenshtein(token, candidate), candidate))
        .collect::<Vec<_>>();
    scored.sort_by_key(|(score, candidate)| (*score, *candidate));
    [
        scored.first().map_or("", |(_, candidate)| *candidate),
        scored.get(1).map_or("", |(_, candidate)| *candidate),
        scored.get(2).map_or("", |(_, candidate)| *candidate),
    ]
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
mod baseline_tests {
    include!("theme_tests.rs");
}
