use crate::style::Color;

/// A 12-step color scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorScale {
    /// The 12 colors of the scale, with step 1 at index 0.
    pub(crate) steps: [Color; 12],
    /// The color for content drawn on step 9, the solid fill.
    pub(crate) on_solid: Color,
}

impl ColorScale {
    /// Creates a scale from 12 colors and picks a dark or white `on_solid` color from step 9.
    #[must_use]
    pub const fn from_steps(steps: [Color; 12]) -> Self {
        let on_solid = heuristic_on_solid(steps[8]);
        Self { steps, on_solid }
    }

    /// Sets the color for content on the solid step, in place of the color picked from step 9.
    #[must_use]
    pub const fn on_solid(mut self, on_solid: Color) -> Self {
        self.on_solid = on_solid;
        self
    }

    /// Returns step `n` of the scale.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "indices are computed from the same lengths that sized the buffer"
    )]
    pub const fn step(&self, n: u8) -> Color {
        let idx = if n == 0 { 0 } else { (n - 1) as usize };
        let clamped = if idx > 11 { 11 } else { idx };
        self.steps[clamped]
    }

    /// Returns all 12 steps of the scale from step 1 to step 12.
    #[must_use]
    pub const fn steps(&self) -> &[Color; 12] {
        &self.steps
    }

    /// Returns the foreground color drawn on step 9.
    #[must_use]
    pub const fn solid_ink(&self) -> Color {
        self.on_solid
    }
}

const fn heuristic_on_solid(step9: Color) -> Color {
    let sum = step9.r as u16 + step9.g as u16 + step9.b as u16;
    if sum > 420 {
        Color::rgb(0x1C, 0x20, 0x24)
    } else {
        Color::rgb(0xFF, 0xFF, 0xFF)
    }
}

kittest_pub! {
    /// The 12-step slate gray scale of the light theme.
    #[must_use]
    pub const fn slate_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFC, 0xFC, 0xFD),
            Color::rgb(0xF9, 0xF9, 0xFB),
            Color::rgb(0xEF, 0xF0, 0xF3),
            Color::rgb(0xE7, 0xE8, 0xEC),
            Color::rgb(0xE0, 0xE1, 0xE6),
            Color::rgb(0xD8, 0xD9, 0xE0),
            Color::rgb(0xCD, 0xCE, 0xD7),
            Color::rgb(0xB9, 0xBB, 0xC6),
            Color::rgb(0x8B, 0x8D, 0x98),
            Color::rgb(0x80, 0x82, 0x8D),
            Color::rgb(0x60, 0x64, 0x6C),
            Color::rgb(0x1C, 0x20, 0x24),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

/// The 12-step iris scale for a light background; no built-in theme uses it.
#[must_use]
#[cfg(feature = "kittest")]
pub const fn iris_light() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0xFD, 0xFD, 0xFF),
        Color::rgb(0xF8, 0xF8, 0xFF),
        Color::rgb(0xF0, 0xF1, 0xFE),
        Color::rgb(0xE6, 0xE7, 0xFF),
        Color::rgb(0xDA, 0xDC, 0xFF),
        Color::rgb(0xCA, 0xCD, 0xFF),
        Color::rgb(0xB5, 0xB8, 0xFF),
        Color::rgb(0x9B, 0x9E, 0xF0),
        Color::rgb(0x5B, 0x5B, 0xD6),
        Color::rgb(0x51, 0x51, 0xCD),
        Color::rgb(0x57, 0x53, 0xC6),
        Color::rgb(0x27, 0x29, 0x62),
    ])
    .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
}

kittest_pub! {
    /// The 12-step green scale of the light theme.
    #[must_use]
    pub const fn green_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFB, 0xFE, 0xFC),
            Color::rgb(0xF4, 0xFB, 0xF6),
            Color::rgb(0xE6, 0xF6, 0xEB),
            Color::rgb(0xD6, 0xF1, 0xDF),
            Color::rgb(0xC4, 0xE8, 0xD1),
            Color::rgb(0xAD, 0xDD, 0xC0),
            Color::rgb(0x8E, 0xCE, 0xAA),
            Color::rgb(0x5B, 0xB9, 0x8C),
            Color::rgb(0x30, 0xA4, 0x6C),
            Color::rgb(0x2B, 0x9A, 0x66),
            Color::rgb(0x1F, 0x7B, 0x53),
            Color::rgb(0x19, 0x3B, 0x2D),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

kittest_pub! {
    /// The 12-step amber scale of the light theme.
    #[must_use]
    pub const fn amber_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFE, 0xFD, 0xFB),
            Color::rgb(0xFE, 0xFA, 0xE4),
            Color::rgb(0xFF, 0xF1, 0xC1),
            Color::rgb(0xFD, 0xE5, 0x88),
            Color::rgb(0xF6, 0xD6, 0x73),
            Color::rgb(0xE9, 0xC0, 0x62),
            Color::rgb(0xE2, 0xB0, 0x46),
            Color::rgb(0xCB, 0x8E, 0x1E),
            Color::rgb(0xFF, 0xC5, 0x3D),
            Color::rgb(0xFF, 0xBA, 0x18),
            Color::rgb(0x9F, 0x5D, 0x00),
            Color::rgb(0x4F, 0x34, 0x22),
        ])
        .on_solid(Color::rgb(0x4F, 0x34, 0x22))
    }
}

kittest_pub! {
    /// The 12-step ruby red scale of the light theme.
    #[must_use]
    pub const fn ruby_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFF, 0xFC, 0xFD),
            Color::rgb(0xFF, 0xF7, 0xF8),
            Color::rgb(0xFE, 0xEA, 0xED),
            Color::rgb(0xFF, 0xDC, 0xE1),
            Color::rgb(0xFF, 0xCD, 0xD4),
            Color::rgb(0xF8, 0xBB, 0xC4),
            Color::rgb(0xEF, 0xA5, 0xB1),
            Color::rgb(0xE5, 0x84, 0x96),
            Color::rgb(0xE5, 0x48, 0x66),
            Color::rgb(0xDC, 0x3B, 0x5D),
            Color::rgb(0xCA, 0x24, 0x4D),
            Color::rgb(0x64, 0x17, 0x23),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

kittest_pub! {
    /// The 12-step cyan scale of the light theme.
    #[must_use]
    pub const fn cyan_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFA, 0xFD, 0xFE),
            Color::rgb(0xF2, 0xFA, 0xFB),
            Color::rgb(0xDE, 0xF7, 0xF9),
            Color::rgb(0xCA, 0xF1, 0xF6),
            Color::rgb(0xB5, 0xE9, 0xF0),
            Color::rgb(0x9D, 0xDD, 0xE7),
            Color::rgb(0x7D, 0xCE, 0xDC),
            Color::rgb(0x3D, 0xB9, 0xCC),
            Color::rgb(0x00, 0xA2, 0xC7),
            Color::rgb(0x07, 0x97, 0xB9),
            Color::rgb(0x0F, 0x76, 0x90),
            Color::rgb(0x0D, 0x34, 0x42),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

kittest_pub! {
    /// The 12-step slate gray scale of the dark theme.
    #[must_use]
    pub const fn slate_dark() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0x11, 0x11, 0x13),
            Color::rgb(0x18, 0x19, 0x1B),
            Color::rgb(0x21, 0x22, 0x25),
            Color::rgb(0x27, 0x2A, 0x2D),
            Color::rgb(0x2E, 0x31, 0x35),
            Color::rgb(0x36, 0x3A, 0x3F),
            Color::rgb(0x43, 0x48, 0x4E),
            Color::rgb(0x5A, 0x61, 0x69),
            Color::rgb(0x69, 0x6E, 0x77),
            Color::rgb(0x77, 0x7B, 0x84),
            Color::rgb(0xB0, 0xB4, 0xBA),
            Color::rgb(0xED, 0xEE, 0xF0),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

/// The 12-step iris scale for a dark background; no built-in theme uses it.
#[must_use]
#[cfg(feature = "kittest")]
pub const fn iris_dark() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0x13, 0x13, 0x1E),
        Color::rgb(0x17, 0x16, 0x25),
        Color::rgb(0x20, 0x22, 0x48),
        Color::rgb(0x26, 0x2A, 0x65),
        Color::rgb(0x30, 0x33, 0x74),
        Color::rgb(0x3D, 0x3E, 0x82),
        Color::rgb(0x4A, 0x4A, 0x95),
        Color::rgb(0x59, 0x58, 0xB1),
        Color::rgb(0x5B, 0x5B, 0xD6),
        Color::rgb(0x6E, 0x6A, 0xDE),
        Color::rgb(0xB1, 0xA9, 0xFF),
        Color::rgb(0xE1, 0xDE, 0xFF),
    ])
    .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
}

kittest_pub! {
    /// The 12-step blue scale of the light theme.
    #[must_use]
    pub const fn blue_light() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0xFB, 0xFC, 0xFE),
            Color::rgb(0xF5, 0xF8, 0xFC),
            Color::rgb(0xEA, 0xF1, 0xFA),
            Color::rgb(0xDD, 0xE9, 0xF8),
            Color::rgb(0xCF, 0xE0, 0xF5),
            Color::rgb(0xBA, 0xD2, 0xEF),
            Color::rgb(0x9D, 0xBF, 0xE6),
            Color::rgb(0x7A, 0xA5, 0xD8),
            Color::rgb(0x3D, 0x6B, 0xB3),
            Color::rgb(0x35, 0x61, 0x9F),
            Color::rgb(0x2F, 0x5B, 0xA8),
            Color::rgb(0x1E, 0x3A, 0x6B),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

kittest_pub! {
    /// The 12-step blue scale of the dark theme.
    #[must_use]
    pub const fn blue_dark() -> ColorScale {
        ColorScale::from_steps([
            Color::rgb(0x10, 0x13, 0x19),
            Color::rgb(0x15, 0x1A, 0x24),
            Color::rgb(0x1C, 0x27, 0x40),
            Color::rgb(0x22, 0x31, 0x52),
            Color::rgb(0x2A, 0x3C, 0x63),
            Color::rgb(0x33, 0x48, 0x76),
            Color::rgb(0x3E, 0x57, 0x8C),
            Color::rgb(0x4C, 0x68, 0xA4),
            Color::rgb(0x3D, 0x6B, 0xB3),
            Color::rgb(0x4C, 0x7C, 0xC4),
            Color::rgb(0x8F, 0xB4, 0xE8),
            Color::rgb(0xD8, 0xE4, 0xF8),
        ])
        .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
    }
}

/// The 12-step green scale of the dark theme.
#[must_use]
pub(crate) const fn green_dark() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0x0D, 0x14, 0x10),
        Color::rgb(0x0F, 0x1B, 0x16),
        Color::rgb(0x0D, 0x27, 0x1E),
        Color::rgb(0x0B, 0x33, 0x26),
        Color::rgb(0x10, 0x3F, 0x30),
        Color::rgb(0x1A, 0x4D, 0x3B),
        Color::rgb(0x24, 0x5E, 0x49),
        Color::rgb(0x2D, 0x76, 0x5A),
        Color::rgb(0x30, 0xA4, 0x6C),
        Color::rgb(0x33, 0xB0, 0x74),
        Color::rgb(0x4C, 0xC3, 0x8A),
        Color::rgb(0xD2, 0xF7, 0xE3),
    ])
    .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
}

/// The 12-step amber scale of the dark theme.
#[must_use]
pub(crate) const fn amber_dark() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0x16, 0x11, 0x0A),
        Color::rgb(0x1D, 0x18, 0x0E),
        Color::rgb(0x30, 0x21, 0x0C),
        Color::rgb(0x3F, 0x2A, 0x0C),
        Color::rgb(0x4D, 0x33, 0x0D),
        Color::rgb(0x5E, 0x40, 0x14),
        Color::rgb(0x73, 0x4F, 0x18),
        Color::rgb(0x8E, 0x62, 0x1A),
        Color::rgb(0xFF, 0xC5, 0x3D),
        Color::rgb(0xFF, 0xD0, 0x60),
        Color::rgb(0xFF, 0xCA, 0x68),
        Color::rgb(0xFF, 0xE8, 0xC2),
    ])
    .on_solid(Color::rgb(0x4F, 0x34, 0x22))
}

/// The 12-step ruby red scale of the dark theme.
#[must_use]
pub(crate) const fn ruby_dark() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0x19, 0x0E, 0x10),
        Color::rgb(0x21, 0x12, 0x16),
        Color::rgb(0x37, 0x14, 0x1F),
        Color::rgb(0x46, 0x16, 0x26),
        Color::rgb(0x54, 0x1B, 0x2D),
        Color::rgb(0x66, 0x24, 0x36),
        Color::rgb(0x7E, 0x2F, 0x42),
        Color::rgb(0xA1, 0x3A, 0x52),
        Color::rgb(0xE5, 0x48, 0x66),
        Color::rgb(0xEC, 0x5C, 0x78),
        Color::rgb(0xF5, 0x82, 0x9A),
        Color::rgb(0xFE, 0xD2, 0xDE),
    ])
    .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
}

/// The 12-step cyan scale of the dark theme.
#[must_use]
pub(crate) const fn cyan_dark() -> ColorScale {
    ColorScale::from_steps([
        Color::rgb(0x07, 0x12, 0x18),
        Color::rgb(0x07, 0x18, 0x21),
        Color::rgb(0x06, 0x25, 0x31),
        Color::rgb(0x04, 0x30, 0x3F),
        Color::rgb(0x06, 0x3B, 0x4D),
        Color::rgb(0x09, 0x4A, 0x5E),
        Color::rgb(0x0D, 0x5D, 0x73),
        Color::rgb(0x13, 0x75, 0x8D),
        Color::rgb(0x00, 0xA2, 0xC7),
        Color::rgb(0x23, 0xAF, 0xD0),
        Color::rgb(0x4C, 0xCC, 0xE6),
        Color::rgb(0xB6, 0xEC, 0xF7),
    ])
    .on_solid(Color::rgb(0xFF, 0xFF, 0xFF))
}

#[cfg(test)]
#[path = "scale_tests.rs"]
mod tests;
