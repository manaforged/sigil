use crate::layout::style_data::Length;

/// A step of the spacing scale, for `gap`, `padding`, and `margin`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum Space {
    /// No space.
    #[default]
    None,
    /// 2 px, for hairline offsets.
    Xxs,
    /// 4 px.
    Xs,
    /// 8 px.
    Sm,
    /// 12 px.
    Md,
    /// 16 px.
    Lg,
    /// 24 px.
    Xl,
    /// 32 px.
    Xxl,
    /// 48 px.
    Xxxl,
    #[doc(hidden)]
    Raw(Length),
}

impl Space {
    /// A size off the scale, with the reason it cannot be a step.
    #[must_use]
    pub const fn raw(px: f32, _reason: &'static str) -> Self {
        Self::Raw(Length::Px(px))
    }

    #[must_use]
    pub(crate) const fn px(px: f32) -> Self {
        Self::Raw(Length::Px(px))
    }
}

impl From<Space> for Length {
    fn from(space: Space) -> Self {
        match space {
            Space::None => Self::Px(0.0),
            Space::Xxs => Self::Px(2.0),
            Space::Xs => Self::Px(4.0),
            Space::Sm => Self::Px(8.0),
            Space::Md => Self::Px(12.0),
            Space::Lg => Self::Px(16.0),
            Space::Xl => Self::Px(24.0),
            Space::Xxl => Self::Px(32.0),
            Space::Xxxl => Self::Px(48.0),
            Space::Raw(dimension) => dimension,
        }
    }
}

impl From<Length> for Space {
    fn from(dimension: Length) -> Self {
        Self::Raw(dimension)
    }
}
