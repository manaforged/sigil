use crate::style::space;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Density {
    Grid,
    Compact,
    #[default]
    Default,
}

impl Density {
    pub(super) const fn row(self) -> f32 {
        match self {
            Self::Grid => space(6),
            Self::Compact => space(7),
            Self::Default => space(9),
        }
    }

    pub(super) const fn header(self) -> f32 {
        match self {
            Self::Grid => space(7),
            Self::Compact => space(8),
            Self::Default => space(9),
        }
    }

    pub(super) const fn pad_x(self) -> f32 {
        match self {
            Self::Grid | Self::Compact => space(2),
            Self::Default => space(3),
        }
    }

    pub(super) const fn text(self) -> &'static str {
        match self {
            Self::Grid => "text-xs",
            Self::Compact | Self::Default => "text-sm",
        }
    }
}
