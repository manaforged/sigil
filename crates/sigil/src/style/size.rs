use crate::style::{Dimension, Px};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlSize {
    Compact,
    Default,
    Wide,
    Grow,
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
