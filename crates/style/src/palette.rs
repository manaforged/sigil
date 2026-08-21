//! Arcane brand palettes — six-colour role vocabulary applied to any
//! [`StyleVariant`](crate::StyleVariant) to produce a [`Theme`](crate::Theme).
//!
//! Each palette is a fixed set of six role colours:
//! `background`, `text`, `primary`, `success`, `warning`, `danger`.
//! All other surface and hover colours derive from these via simple
//! shading; future work will swap the shading function for an
//! Oklch-based one that handles light/dark variants cleanly.
//!
//! Palette names follow the author Technologies' arcane theme. The
//! aesthetic inspirations behind each one are documented in the plan
//! file, not in identifier names — no community theme brand is
//! lifted.

mod hsv;

use crate::Rgba8;
use hsv::{perceived_brightness, tint_toward_hue};

/// Six-colour role palette. Combine with a
/// [`StyleVariant`](crate::StyleVariant) to assemble a full theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    /// Page / window background.
    pub background: Rgba8,
    /// Default readable text on `background`.
    pub text: Rgba8,
    /// Primary action / accent colour.
    pub primary: Rgba8,
    /// Positive / confirmation colour.
    pub success: Rgba8,
    /// Cautionary / non-blocking warning colour.
    pub warning: Rgba8,
    /// Destructive / error colour.
    pub danger: Rgba8,
}

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

impl Palette {
    /// Clean ethereal light. Bright neutral surfaces with cool-blue
    /// primaries — the "Aetherlight" palette.
    #[must_use]
    pub const fn aetherlight() -> Self {
        Self {
            background: rgb(0xF5, 0xF7, 0xFB),
            text: rgb(0x14, 0x1B, 0x2D),
            primary: rgb(0x33, 0x70, 0xFF),
            success: rgb(0x10, 0xA3, 0x70),
            warning: rgb(0xE0, 0x97, 0x10),
            danger: rgb(0xD0, 0x3A, 0x4A),
        }
    }

    /// Neutral volcanic-glass dark — the default Sigil look.
    /// `Theme::normal()` resolves to this palette under the Normal
    /// variant.
    #[must_use]
    pub const fn obsidian() -> Self {
        Self {
            background: rgb(0x14, 0x16, 0x1C),
            text: rgb(0xE8, 0xEA, 0xF0),
            primary: rgb(0x6E, 0x8E, 0xFF),
            success: rgb(0x4A, 0xD8, 0x9C),
            warning: rgb(0xF2, 0xB6, 0x4B),
            danger: rgb(0xE4, 0x5E, 0x6E),
        }
    }

    /// Saturated green-on-near-black, runes-carved-in-stone.
    /// `Theme::terminal()` resolves to this palette under the Terminal
    /// variant — the second locked regression baseline.
    #[must_use]
    pub const fn terminal() -> Self {
        Self {
            background: rgb(0x0A, 0x0A, 0x0A),
            text: rgb(0x33, 0xFF, 0x33),
            primary: rgb(0x33, 0xFF, 0x33),
            success: rgb(0x33, 0xFF, 0x33),
            warning: rgb(0xFF, 0xAA, 0x00),
            danger: rgb(0xFF, 0x33, 0x33),
        }
    }

    /// Arcane neon-on-near-black with electric-rune accents.
    #[must_use]
    pub const fn voltglyph() -> Self {
        Self {
            background: rgb(0x07, 0x05, 0x12),
            text: rgb(0xE2, 0xE6, 0xFA),
            primary: rgb(0xFF, 0x2A, 0xC8),
            success: rgb(0x2A, 0xF8, 0xC0),
            warning: rgb(0xFF, 0xD0, 0x2A),
            danger: rgb(0xFF, 0x3C, 0x66),
        }
    }

    /// Silvery-purple dark, enchanted-metal sheen.
    #[must_use]
    pub const fn mythril() -> Self {
        Self {
            background: rgb(0x1A, 0x14, 0x26),
            text: rgb(0xE5, 0xDC, 0xF5),
            primary: rgb(0xB0, 0x8C, 0xFF),
            success: rgb(0x80, 0xE0, 0xB4),
            warning: rgb(0xE8, 0xC0, 0x6A),
            danger: rgb(0xEC, 0x6D, 0x96),
        }
    }

    /// Cool blue-grey, icy-fortress muted.
    #[must_use]
    pub const fn frostkeep() -> Self {
        Self {
            background: rgb(0x1C, 0x24, 0x30),
            text: rgb(0xD8, 0xE0, 0xEC),
            primary: rgb(0x6A, 0xB4, 0xE8),
            success: rgb(0x88, 0xCC, 0xB8),
            warning: rgb(0xE8, 0xC8, 0x7A),
            danger: rgb(0xD8, 0x7E, 0x9A),
        }
    }

    /// Warm beige parchment light.
    #[must_use]
    pub const fn vellum() -> Self {
        Self {
            background: rgb(0xF2, 0xE8, 0xD0),
            text: rgb(0x3A, 0x2A, 0x1A),
            primary: rgb(0xB8, 0x68, 0x2A),
            success: rgb(0x5C, 0x8C, 0x4A),
            warning: rgb(0xCC, 0x90, 0x20),
            danger: rgb(0xB0, 0x40, 0x40),
        }
    }

    /// Warm dark with amber fire-lit accents.
    #[must_use]
    pub const fn embergrove() -> Self {
        Self {
            background: rgb(0x1F, 0x16, 0x10),
            text: rgb(0xF0, 0xDC, 0xB4),
            primary: rgb(0xFF, 0xA8, 0x4A),
            success: rgb(0x9C, 0xC8, 0x5C),
            warning: rgb(0xF0, 0xD0, 0x60),
            danger: rgb(0xE0, 0x60, 0x4A),
        }
    }

    /// Oxidised teal-green saturated, ancient-bronze patina.
    #[must_use]
    pub const fn verdigris() -> Self {
        Self {
            background: rgb(0x18, 0x24, 0x22),
            text: rgb(0xD0, 0xE0, 0xC0),
            primary: rgb(0x60, 0xC4, 0xA0),
            success: rgb(0x88, 0xD0, 0x70),
            warning: rgb(0xE0, 0xC4, 0x60),
            danger: rgb(0xD0, 0x70, 0x60),
        }
    }

    /// Soft pastel dark with rose-and-violet bloom.
    #[must_use]
    pub const fn faewine() -> Self {
        Self {
            background: rgb(0x22, 0x1C, 0x2A),
            text: rgb(0xF0, 0xE0, 0xEC),
            primary: rgb(0xE8, 0x96, 0xC8),
            success: rgb(0xA8, 0xD8, 0xC0),
            warning: rgb(0xE8, 0xC8, 0x8C),
            danger: rgb(0xE0, 0x7C, 0x96),
        }
    }

    /// Vibrant dark with crystal-like neon highlights.
    #[must_use]
    pub const fn crystalline() -> Self {
        Self {
            background: rgb(0x0E, 0x10, 0x20),
            text: rgb(0xE0, 0xE8, 0xFA),
            primary: rgb(0x4A, 0xC0, 0xFF),
            success: rgb(0x60, 0xE8, 0xB0),
            warning: rgb(0xFF, 0xD8, 0x60),
            danger: rgb(0xFF, 0x70, 0xA0),
        }
    }

    /// the host engine a full palette from a single accent colour — Spellforge.
    ///
    /// All six role colours are derived from the accent:
    /// - `primary` is the accent itself.
    /// - `background` / `text` are chosen for legible contrast (dark
    ///   surface for a bright accent, light surface for a dark one).
    /// - `success` / `warning` / `danger` retain the accent's
    ///   saturation and value but rotate their hue toward the
    ///   canonical role hues (green ≈ 130°, amber ≈ 45°, red ≈ 5°)
    ///   so they stay recognisable as success/warning/danger while
    ///   feeling like part of the same palette family.
    #[must_use]
    pub fn spellforge(accent: Rgba8) -> Self {
        let accent_is_bright = perceived_brightness(accent) > 128;
        let (bg, fg) = if accent_is_bright {
            (rgb(0x14, 0x16, 0x1C), rgb(0xE8, 0xEA, 0xF0))
        } else {
            (rgb(0xF5, 0xF7, 0xFB), rgb(0x14, 0x1B, 0x2D))
        };
        Self {
            background: bg,
            text: fg,
            primary: accent,
            success: tint_toward_hue(accent, 130.0),
            warning: tint_toward_hue(accent, 45.0),
            danger: tint_toward_hue(accent, 5.0),
        }
    }
}

#[cfg(test)]
#[path = "palette_tests.rs"]
mod tests;
