//! Polymorphic colour input + atom helpers for the `Styled` surface.

use crate::{ColorAtom, Rgba8, StyleAtom};

/// Polymorphic colour argument accepted by [`super::Styled::bg`] and
/// [`super::Styled::text_color`].
///
/// Construct via the `Into` impls — pass an [`Rgba8`], a `&'static str`
/// theme token, or a [`ColorAtom`] and the implementations route to the
/// right atom variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOrToken {
    /// Concrete sRGB value.
    Rgba(Rgba8),
    /// Theme-token reference resolved at paint time.
    Token(&'static str),
    /// A pre-built colour atom.
    Atom(ColorAtom),
}

impl From<Rgba8> for ColorOrToken {
    fn from(value: Rgba8) -> Self {
        Self::Rgba(value)
    }
}

impl From<&'static str> for ColorOrToken {
    fn from(value: &'static str) -> Self {
        Self::Token(value)
    }
}

impl From<ColorAtom> for ColorOrToken {
    fn from(value: ColorAtom) -> Self {
        Self::Atom(value)
    }
}

pub(super) const fn background_atom(color: ColorOrToken) -> StyleAtom {
    match color {
        ColorOrToken::Rgba(rgba) => StyleAtom::BackgroundColor(ColorAtom::Rgba8(rgba)),
        ColorOrToken::Token(token) => StyleAtom::BackgroundThemeRef(token),
        ColorOrToken::Atom(atom) => StyleAtom::BackgroundColor(atom),
    }
}

pub(super) const fn text_atom(color: ColorOrToken) -> StyleAtom {
    match color {
        ColorOrToken::Rgba(rgba) => StyleAtom::TextColor(ColorAtom::Rgba8(rgba)),
        ColorOrToken::Token(token) => StyleAtom::TextThemeRef(token),
        ColorOrToken::Atom(atom) => StyleAtom::TextColor(atom),
    }
}
