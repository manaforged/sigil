use crate::{Dimension, Px};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    Compact,
    Default,
    Wide,
    Grow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPercentage {
    Px(Px),
    Percent(f32),
}

impl LengthPercentage {
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
