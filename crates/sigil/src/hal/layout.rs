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

/// The space a layout may use on each axis: a fixed length, a limit, or the content size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub(crate) width: Dim,
    pub(crate) height: Dim,
}

impl Constraint {
    /// Creates a constraint with a fixed width and height.
    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dim::fixed(width),
            height: Dim::fixed(height),
        }
    }

    /// A constraint that asks for the max-content size on both axes.
    #[must_use]
    pub const fn max_content() -> Self {
        Self {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        }
    }

    /// A constraint that asks for the min-content size on both axes.
    #[must_use]
    pub const fn min_content() -> Self {
        Self {
            width: Dim::MinContent,
            height: Dim::MinContent,
        }
    }

    /// Makes the width fixed at `width` and keeps the height constraint.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Dim::Fixed(width);
        self
    }

    /// The width when it is fixed; `None` for a content-sized width.
    #[must_use]
    pub const fn fixed_width(self) -> Option<f32> {
        match self.width {
            Dim::Fixed(value) => Some(value),
            Dim::MinContent | Dim::MaxContent => None,
        }
    }

    /// The height when it is fixed; `None` for a content-sized height.
    #[must_use]
    pub const fn fixed_height(self) -> Option<f32> {
        match self.height {
            Dim::Fixed(value) => Some(value),
            Dim::MinContent | Dim::MaxContent => None,
        }
    }
}
