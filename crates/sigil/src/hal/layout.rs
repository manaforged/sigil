#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dim {
    Fixed(f32),
    MinContent,
    MaxContent,
}

impl Dim {
    #[must_use]
    pub const fn fixed(value: f32) -> Self {
        Self::Fixed(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: Dim,
    pub height: Dim,
}

impl Size {
    #[must_use]
    pub const fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dim::Fixed(width),
            height: Dim::Fixed(height),
        }
    }

    #[must_use]
    pub const fn max_content() -> Self {
        Self {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        }
    }

    #[must_use]
    pub const fn min_content() -> Self {
        Self {
            width: Dim::MinContent,
            height: Dim::MinContent,
        }
    }
}
