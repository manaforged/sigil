#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Dim {
    Fixed(f32),
    MinContent,
    MaxContent,
}

impl Dim {
    #[must_use]
    pub(crate) const fn fixed(value: f32) -> Self {
        Self::Fixed(value)
    }
}

/// A size constraint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub(crate) width: Dim,
    pub(crate) height: Dim,
}

impl Size {
    /// Creates a size with both dimensions definite.
    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dim::fixed(width),
            height: Dim::fixed(height),
        }
    }

    /// Returns the max content size.
    #[must_use]
    pub const fn max_content() -> Self {
        Self {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        }
    }

    /// Returns the min content size.
    #[must_use]
    pub const fn min_content() -> Self {
        Self {
            width: Dim::MinContent,
            height: Dim::MinContent,
        }
    }

    /// Sets the width.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Dim::Fixed(width);
        self
    }

    /// Returns the fixed width.
    #[must_use]
    pub const fn fixed_width(self) -> Option<f32> {
        match self.width {
            Dim::Fixed(value) => Some(value),
            Dim::MinContent | Dim::MaxContent => None,
        }
    }

    /// Returns the fixed height.
    #[must_use]
    pub const fn fixed_height(self) -> Option<f32> {
        match self.height {
            Dim::Fixed(value) => Some(value),
            Dim::MinContent | Dim::MaxContent => None,
        }
    }
}
