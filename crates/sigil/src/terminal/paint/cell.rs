use crate::style::{Rgba8, Tone};
use smol_str::SmolStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ink {
    #[default]
    Bare,
    Of(Tone),
    Exact(Rgba8),
}

impl Ink {
    pub(super) const fn pick(colour: Rgba8, tone: Option<Tone>) -> Self {
        match (colour, tone) {
            (colour, _) if colour.a > 0 => Self::Exact(colour),
            (_, Some(tone)) => Self::Of(tone),
            (_, None) => Self::Bare,
        }
    }

    pub(crate) const fn set(self) -> bool {
        !matches!(self, Self::Bare)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Underline {
    #[default]
    None,
    Straight,
    Curly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    pub underline: Underline,
    pub strike: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub text: SmolStr,
    pub fg: Ink,
    pub bg: Ink,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub lines: Lines,
    pub underline_ink: Ink,
    pub link: Option<SmolStr>,
}

impl Cell {
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            text: SmolStr::new_inline(" "),
            fg: Ink::Bare,
            bg: Ink::Bare,
            bold: false,
            italic: false,
            dim: false,
            lines: Lines {
                underline: Underline::None,
                strike: false,
            },
            underline_ink: Ink::Bare,
            link: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(in crate::terminal) struct Face {
    pub(in crate::terminal) bold: bool,
    pub(in crate::terminal) italic: bool,
    pub(in crate::terminal) dim: bool,
    pub(in crate::terminal) underline: Underline,
}
