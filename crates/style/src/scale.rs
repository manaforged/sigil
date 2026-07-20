//! 12-step color scales — the foundation of the DS1 token system.
//!
//! Each named hue (gray, accent, success, warn, error, info) is a
//! 12-step scale following the Radix Colors convention:
//!
//! | Step  | Role                                         |
//! |-------|----------------------------------------------|
//! |  1    | App background (lightest in light themes)    |
//! |  2    | Subtle background (cards, panels)            |
//! |  3    | UI element rest (button face, input rest)    |
//! |  4    | UI element hover                             |
//! |  5    | UI element pressed / active                  |
//! |  6    | Subtle border                                |
//! |  7    | Border (default)                             |
//! |  8    | Border hover / focused                       |
//! |  9    | Solid (the saturated brand color)            |
//! | 10    | Solid hover                                  |
//! | 11    | Low-contrast text (muted)                    |
//! | 12    | High-contrast text (default body)            |
//!
//! Widgets reference scales by *role* (`Gray`, `Accent`, ...) plus a
//! step number. Themes can swap any scale to recolor everything that
//! references that role, or override individual steps for a custom
//! brand look.

use crate::Rgba8;

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

/// A 12-step color scale.
///
/// Indexed 1..=12 (Radix Colors convention). The accessor `step`
/// clamps out-of-range inputs to the nearest valid step.
///
/// `on_solid` is the WCAG-AA-passing text color when painting on
/// `step(9)` (the saturated solid). Most scales use white; bright
/// warm scales (amber, yellow) use a dark color because white on
/// yellow fails contrast. The default for a scale built from
/// `from_steps` alone is a heuristic — explicit `.with_on_solid(...)`
/// is the right call for built-in or brand scales where the contrast
/// color is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorScale {
    /// The twelve steps, indexed from 1 (step at `[0]`) to 12 (step at `[11]`).
    pub steps: [Rgba8; 12],
    /// The text color that reads correctly on `step(9)`.
    pub on_solid: Rgba8,
}

impl ColorScale {
    /// Build a scale from explicit step values. The `on_solid` color
    /// defaults to a luminance-based heuristic — call
    /// [`with_on_solid`](Self::with_on_solid) to set it explicitly
    /// when the right contrast color is known.
    #[must_use]
    pub const fn from_steps(steps: [Rgba8; 12]) -> Self {
        let on_solid = heuristic_on_solid(steps[8]);
        Self { steps, on_solid }
    }

    /// Override the `on_solid` contrast color.
    #[must_use]
    pub const fn with_on_solid(mut self, on_solid: Rgba8) -> Self {
        self.on_solid = on_solid;
        self
    }

    /// Read step `n` (1..=12, clamped).
    #[must_use]
    #[allow(clippy::indexing_slicing)] // clamped to 0..=11 above; .get() not available in const fn
    pub const fn step(&self, n: u8) -> Rgba8 {
        let idx = if n == 0 { 0 } else { (n - 1) as usize };
        let clamped = if idx > 11 { 11 } else { idx };
        self.steps[clamped]
    }
}

/// Heuristic on-solid picker — sums step-9 channels and picks
/// black/white based on a perceived-brightness threshold. Used as the
/// default when a scale doesn't call [`ColorScale::with_on_solid`].
const fn heuristic_on_solid(step9: Rgba8) -> Rgba8 {
    let sum = step9.r as u16 + step9.g as u16 + step9.b as u16;
    if sum > 420 {
        rgb(0x1C, 0x20, 0x24)
    } else {
        rgb(0xFF, 0xFF, 0xFF)
    }
}

/// Slate (cool neutral gray) — light theme.
///
/// Radix Colors-inspired. The default `Theme::gray` scale.
#[must_use]
pub const fn slate_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFC, 0xFC, 0xFD),
        rgb(0xF9, 0xF9, 0xFB),
        rgb(0xEF, 0xF0, 0xF3),
        rgb(0xE7, 0xE8, 0xEC),
        rgb(0xE0, 0xE1, 0xE6),
        rgb(0xD8, 0xD9, 0xE0),
        rgb(0xCD, 0xCE, 0xD7),
        rgb(0xB9, 0xBB, 0xC6),
        rgb(0x8B, 0x8D, 0x98),
        rgb(0x80, 0x82, 0x8D),
        rgb(0x60, 0x64, 0x6C),
        rgb(0x1C, 0x20, 0x24),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Iris (cool blue-purple) — light theme accent default.
#[must_use]
pub const fn iris_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFD, 0xFD, 0xFF),
        rgb(0xF8, 0xF8, 0xFF),
        rgb(0xF0, 0xF1, 0xFE),
        rgb(0xE6, 0xE7, 0xFF),
        rgb(0xDA, 0xDC, 0xFF),
        rgb(0xCA, 0xCD, 0xFF),
        rgb(0xB5, 0xB8, 0xFF),
        rgb(0x9B, 0x9E, 0xF0),
        rgb(0x5B, 0x5B, 0xD6),
        rgb(0x51, 0x51, 0xCD),
        rgb(0x57, 0x53, 0xC6),
        rgb(0x27, 0x29, 0x62),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Green (success) — light theme.
#[must_use]
pub const fn green_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFB, 0xFE, 0xFC),
        rgb(0xF4, 0xFB, 0xF6),
        rgb(0xE6, 0xF6, 0xEB),
        rgb(0xD6, 0xF1, 0xDF),
        rgb(0xC4, 0xE8, 0xD1),
        rgb(0xAD, 0xDD, 0xC0),
        rgb(0x8E, 0xCE, 0xAA),
        rgb(0x5B, 0xB9, 0x8C),
        rgb(0x30, 0xA4, 0x6C),
        rgb(0x2B, 0x9A, 0x66),
        rgb(0x21, 0x83, 0x58),
        rgb(0x19, 0x3B, 0x2D),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Amber (warn) — light theme.
#[must_use]
pub const fn amber_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFE, 0xFD, 0xFB),
        rgb(0xFE, 0xFA, 0xE4),
        rgb(0xFF, 0xF1, 0xC1),
        rgb(0xFD, 0xE5, 0x88),
        rgb(0xF6, 0xD6, 0x73),
        rgb(0xE9, 0xC0, 0x62),
        rgb(0xE2, 0xB0, 0x46),
        rgb(0xCB, 0x8E, 0x1E),
        rgb(0xFF, 0xC5, 0x3D),
        rgb(0xFF, 0xBA, 0x18),
        rgb(0xAB, 0x64, 0x00),
        rgb(0x4F, 0x34, 0x22),
    ])
    .with_on_solid(rgb(0x4F, 0x34, 0x22))
}

/// Ruby (error / destructive) — light theme.
#[must_use]
pub const fn ruby_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFF, 0xFC, 0xFD),
        rgb(0xFF, 0xF7, 0xF8),
        rgb(0xFE, 0xEA, 0xED),
        rgb(0xFF, 0xDC, 0xE1),
        rgb(0xFF, 0xCD, 0xD4),
        rgb(0xF8, 0xBB, 0xC4),
        rgb(0xEF, 0xA5, 0xB1),
        rgb(0xE5, 0x84, 0x96),
        rgb(0xE5, 0x48, 0x66),
        rgb(0xDC, 0x3B, 0x5D),
        rgb(0xCA, 0x24, 0x4D),
        rgb(0x64, 0x17, 0x23),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Cyan (info) — light theme.
#[must_use]
pub const fn cyan_light() -> ColorScale {
    ColorScale::from_steps([
        rgb(0xFA, 0xFD, 0xFE),
        rgb(0xF2, 0xFA, 0xFB),
        rgb(0xDE, 0xF7, 0xF9),
        rgb(0xCA, 0xF1, 0xF6),
        rgb(0xB5, 0xE9, 0xF0),
        rgb(0x9D, 0xDD, 0xE7),
        rgb(0x7D, 0xCE, 0xDC),
        rgb(0x3D, 0xB9, 0xCC),
        rgb(0x00, 0xA2, 0xC7),
        rgb(0x07, 0x97, 0xB9),
        rgb(0x10, 0x7D, 0x98),
        rgb(0x0D, 0x34, 0x42),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Slate (cool neutral gray) — dark theme.
///
/// Step 1 is the darkest (app background), step 12 the lightest
/// (default text). All twelve steps stay on the cool-gray hue so a
/// dark theme keeps the same compositional contract as light.
#[must_use]
pub const fn slate_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x11, 0x11, 0x13),
        rgb(0x18, 0x19, 0x1B),
        rgb(0x21, 0x22, 0x25),
        rgb(0x27, 0x2A, 0x2D),
        rgb(0x2E, 0x31, 0x35),
        rgb(0x36, 0x3A, 0x3F),
        rgb(0x43, 0x48, 0x4E),
        rgb(0x5A, 0x61, 0x69),
        rgb(0x69, 0x6E, 0x77),
        rgb(0x77, 0x7B, 0x84),
        rgb(0xB0, 0xB4, 0xBA),
        rgb(0xED, 0xEE, 0xF0),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Iris (cool blue-purple) — dark theme accent default.
#[must_use]
pub const fn iris_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x13, 0x13, 0x1E),
        rgb(0x17, 0x16, 0x25),
        rgb(0x20, 0x22, 0x48),
        rgb(0x26, 0x2A, 0x65),
        rgb(0x30, 0x33, 0x74),
        rgb(0x3D, 0x3E, 0x82),
        rgb(0x4A, 0x4A, 0x95),
        rgb(0x59, 0x58, 0xB1),
        rgb(0x5B, 0x5B, 0xD6),
        rgb(0x6E, 0x6A, 0xDE),
        rgb(0xB1, 0xA9, 0xFF),
        rgb(0xE1, 0xDE, 0xFF),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Green (success) — dark theme.
#[must_use]
pub const fn green_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x0D, 0x14, 0x10),
        rgb(0x0F, 0x1B, 0x16),
        rgb(0x0D, 0x27, 0x1E),
        rgb(0x0B, 0x33, 0x26),
        rgb(0x10, 0x3F, 0x30),
        rgb(0x1A, 0x4D, 0x3B),
        rgb(0x24, 0x5E, 0x49),
        rgb(0x2D, 0x76, 0x5A),
        rgb(0x30, 0xA4, 0x6C),
        rgb(0x33, 0xB0, 0x74),
        rgb(0x4C, 0xC3, 0x8A),
        rgb(0xD2, 0xF7, 0xE3),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Amber (warn) — dark theme.
#[must_use]
pub const fn amber_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x16, 0x11, 0x0A),
        rgb(0x1D, 0x18, 0x0E),
        rgb(0x30, 0x21, 0x0C),
        rgb(0x3F, 0x2A, 0x0C),
        rgb(0x4D, 0x33, 0x0D),
        rgb(0x5E, 0x40, 0x14),
        rgb(0x73, 0x4F, 0x18),
        rgb(0x8E, 0x62, 0x1A),
        rgb(0xFF, 0xC5, 0x3D),
        rgb(0xFF, 0xD0, 0x60),
        rgb(0xFF, 0xCA, 0x68),
        rgb(0xFF, 0xE8, 0xC2),
    ])
    .with_on_solid(rgb(0x4F, 0x34, 0x22))
}

/// Ruby (error / destructive) — dark theme.
#[must_use]
pub const fn ruby_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x19, 0x0E, 0x10),
        rgb(0x21, 0x12, 0x16),
        rgb(0x37, 0x14, 0x1F),
        rgb(0x46, 0x16, 0x26),
        rgb(0x54, 0x1B, 0x2D),
        rgb(0x66, 0x24, 0x36),
        rgb(0x7E, 0x2F, 0x42),
        rgb(0xA1, 0x3A, 0x52),
        rgb(0xE5, 0x48, 0x66),
        rgb(0xEC, 0x5C, 0x78),
        rgb(0xF5, 0x82, 0x9A),
        rgb(0xFE, 0xD2, 0xDE),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Cyan (info) — dark theme.
#[must_use]
pub const fn cyan_dark() -> ColorScale {
    ColorScale::from_steps([
        rgb(0x07, 0x12, 0x18),
        rgb(0x07, 0x18, 0x21),
        rgb(0x06, 0x25, 0x31),
        rgb(0x04, 0x30, 0x3F),
        rgb(0x06, 0x3B, 0x4D),
        rgb(0x09, 0x4A, 0x5E),
        rgb(0x0D, 0x5D, 0x73),
        rgb(0x13, 0x75, 0x8D),
        rgb(0x00, 0xA2, 0xC7),
        rgb(0x23, 0xAF, 0xD0),
        rgb(0x4C, 0xCC, 0xE6),
        rgb(0xB6, 0xEC, 0xF7),
    ])
    .with_on_solid(rgb(0xFF, 0xFF, 0xFF))
}

/// Named color scale roles. Themes carry one scale per role; widgets
/// reference roles by name (`Accent`, `Success`, …) rather than
/// hard-coding a specific hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleRole {
    /// Neutral grays — surfaces, borders, default text.
    Gray,
    /// Brand / accent — primary actions, focus rings.
    Accent,
    /// Success / positive — confirmation chips, valid borders.
    Success,
    /// Warn / cautionary — non-blocking warnings.
    Warn,
    /// Error / destructive — invalid inputs, delete actions.
    Error,
    /// Info / neutral notice — hint banners.
    Info,
}

impl ScaleRole {
    /// Parse a scale role from its lowercase utility-class form
    /// (`"gray"`, `"accent"`, `"success"`, `"warn"`, `"error"`,
    /// `"info"`). Returns `None` for any other input.
    #[must_use]
    pub fn from_token(name: &str) -> Option<Self> {
        match name {
            "gray" => Some(Self::Gray),
            "accent" => Some(Self::Accent),
            "success" => Some(Self::Success),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            "info" => Some(Self::Info),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_indexes_from_one_through_twelve() {
        let scale = slate_light();
        // Step 1 is the lightest, step 12 the darkest, in a light scale.
        assert_eq!(scale.step(1), rgb(0xFC, 0xFC, 0xFD));
        assert_eq!(scale.step(12), rgb(0x1C, 0x20, 0x24));
    }

    #[test]
    fn step_clamps_zero_and_out_of_range() {
        let scale = slate_light();
        assert_eq!(scale.step(0), scale.step(1));
        assert_eq!(scale.step(13), scale.step(12));
        assert_eq!(scale.step(u8::MAX), scale.step(12));
    }

    #[test]
    fn amber_dark_on_solid_is_dark_brown_not_pale_cream() {
        // Regression guard for the WCAG contrast failure that the
        // pre-`on_solid` model produced. `amber_dark().step(12)` is
        // pale cream (rgb 0xFF,0xE8,0xC2) — when used as text on the
        // saturated yellow step-9 it gave ~1.4:1 contrast (WCAG AA
        // requires 4.5:1 for normal text). The `on_solid` field
        // must instead be a dark brown.
        let amber = amber_dark();
        assert_eq!(amber.step(9), rgb(0xFF, 0xC5, 0x3D));
        assert_eq!(amber.on_solid, rgb(0x4F, 0x34, 0x22));
        assert_ne!(
            amber.on_solid,
            amber.step(12),
            "amber on_solid must NOT be step 12 (pale cream) — that fails WCAG AA"
        );
    }

    #[test]
    fn light_scales_with_saturated_step_9_use_white_on_solid() {
        // Most saturated colors (iris, green, ruby, cyan) read white
        // text correctly on step 9. Verify the explicit on_solid
        // assignments match the documented contract.
        let white = rgb(0xFF, 0xFF, 0xFF);
        assert_eq!(iris_light().on_solid, white);
        assert_eq!(green_light().on_solid, white);
        assert_eq!(ruby_light().on_solid, white);
        assert_eq!(cyan_light().on_solid, white);
    }

    #[test]
    fn role_token_round_trips() {
        for (name, role) in [
            ("gray", ScaleRole::Gray),
            ("accent", ScaleRole::Accent),
            ("success", ScaleRole::Success),
            ("warn", ScaleRole::Warn),
            ("error", ScaleRole::Error),
            ("info", ScaleRole::Info),
        ] {
            assert_eq!(ScaleRole::from_token(name), Some(role));
        }
        assert_eq!(ScaleRole::from_token("unknown"), None);
    }
}
