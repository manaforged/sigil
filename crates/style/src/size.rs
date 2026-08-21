//! Theme-resolved widget size vocabulary.

use crate::{Dimension, Px};

/// Semantic size presets shared by Sigil widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    /// Small/tight preset.
    Compact,
    /// Normal preset.
    Default,
    /// Expanded preset.
    Wide,
    /// Fill available space or defer to automatic sizing.
    Grow,
}

/// Axis-independent length expressed as logical pixels or a percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPercentage {
    /// Logical pixels.
    Px(Px),
    /// Fraction in `[0.0, 1.0]` of the containing axis.
    Percent(f32),
}

impl LengthPercentage {
    /// Return the logical-pixel value, or zero for percentage values.
    #[must_use]
    pub const fn px_or_zero(self) -> f32 {
        match self {
            Self::Px(Px(px)) => px as f32,
            Self::Percent(_) => 0.0,
        }
    }
}

impl From<Px> for LengthPercentage {
    fn from(px: Px) -> Self {
        Self::Px(px)
    }
}

impl Dimension {
    /// Return the logical-pixel value, or zero for non-fixed dimensions.
    #[must_use]
    pub const fn px_or_zero(self) -> f32 {
        match self {
            Self::Px(Px(px)) => px as f32,
            Self::Full | Self::Fraction(_) | Self::Auto => 0.0,
        }
    }
}

#[cfg(test)]
#[path = "size_tests.rs"]
mod tests;
