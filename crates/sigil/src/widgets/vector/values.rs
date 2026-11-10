use crate::style::{Rgba8, Theme, Token};

/// Color of `fill` and `stroke`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathColor {
    /// The rgba path color.
    Rgba(Rgba8),
    /// The token path color.
    Token(Token),
}

impl PathColor {
    pub(super) fn resolve(self, theme: &Theme) -> Rgba8 {
        match self {
            Self::Rgba(color) => color,
            Self::Token(token) => theme.color(token),
        }
    }
}

impl From<Rgba8> for PathColor {
    fn from(color: Rgba8) -> Self {
        Self::Rgba(color)
    }
}

impl From<Token> for PathColor {
    fn from(token: Token) -> Self {
        Self::Token(token)
    }
}

/// Path fill paint.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PathFill {
    /// The solid path fill.
    Solid(Rgba8),
    /// The theme path fill.
    Theme(Token),
    /// The linear path fill.
    Linear {
        /// The start.
        start: (f32, f32),
        /// The end.
        end: (f32, f32),
        /// The stops.
        stops: Vec<(f32, Rgba8)>,
    },
    /// The radial path fill.
    Radial {
        /// The center.
        center: (f32, f32),
        /// The radius.
        radius: f32,
        /// The stops.
        stops: Vec<(f32, Rgba8)>,
    },
}
