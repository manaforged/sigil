use crate::{ColorAtom, Rgba8, StyleAtom};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOrToken {
    Rgba(Rgba8),
    Token(&'static str),
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

pub(super) const fn border_color_atom(color: ColorOrToken) -> StyleAtom {
    match color {
        ColorOrToken::Rgba(rgba) => StyleAtom::BorderColor(ColorAtom::Rgba8(rgba)),
        ColorOrToken::Token(token) => StyleAtom::BorderThemeRef(token),
        ColorOrToken::Atom(atom) => StyleAtom::BorderColor(atom),
    }
}

pub(super) const fn text_atom(color: ColorOrToken) -> StyleAtom {
    match color {
        ColorOrToken::Rgba(rgba) => StyleAtom::TextColor(ColorAtom::Rgba8(rgba)),
        ColorOrToken::Token(token) => StyleAtom::TextThemeRef(token),
        ColorOrToken::Atom(atom) => StyleAtom::TextColor(atom),
    }
}
