mod hsv;

use crate::style::Rgba8;
use crate::style::scale::rgb;
use hsv::{perceived_brightness, tint_toward_hue};

pub(crate) const fn is_dark(c: Rgba8) -> bool {
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    (r * 30 + g * 59 + b * 11) < 12_800
}

pub(crate) const fn pick_higher_contrast(against: Rgba8, a: Rgba8, b: Rgba8) -> Rgba8 {
    let target = perceived_brightness_u32(against);
    let da = perceived_brightness_u32(a).abs_diff(target);
    let db = perceived_brightness_u32(b).abs_diff(target);
    if da >= db { a } else { b }
}

pub(crate) const fn perceived_brightness_u32(c: Rgba8) -> u32 {
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    r * 30 + g * 59 + b * 11
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    pub background: Rgba8,
    pub text: Rgba8,
    pub primary: Rgba8,
    pub success: Rgba8,
    pub warning: Rgba8,
    pub danger: Rgba8,
}

impl Palette {
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
