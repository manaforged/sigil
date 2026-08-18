//! Layout constraint types.

/// Available space in one layout axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dim {
    /// A definite length in logical pixels.
    Fixed(f32),
    /// Ask the engine for the minimum intrinsic size.
    MinContent,
    /// Ask the engine for the maximum intrinsic size.
    MaxContent,
}

impl Dim {
    /// Construct a fixed logical-pixel dimension.
    #[must_use]
    pub const fn fixed(value: f32) -> Self {
        Self::Fixed(value)
    }
}

/// Available space passed to layout computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Width constraint.
    pub width: Dim,
    /// Height constraint.
    pub height: Dim,
}

impl Size {
    /// Construct a definite logical-pixel size.
    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dim::Fixed(width),
            height: Dim::Fixed(height),
        }
    }

    /// Construct a max-content size constraint.
    #[must_use]
    pub const fn max_content() -> Self {
        Self {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        }
    }

    /// Construct a min-content size constraint.
    #[must_use]
    pub const fn min_content() -> Self {
        Self {
            width: Dim::MinContent,
            height: Dim::MinContent,
        }
    }
}
