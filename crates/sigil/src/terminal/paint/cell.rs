use crate::style::{Rgba8, Tone};
use smol_str::SmolStr;

/// The color source of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ink {
    /// The bare ink.
    #[default]
    Bare,
    /// The of ink.
    Of(Tone),
    /// The exact ink.
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

/// The underline style of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Underline {
    /// The none underline.
    #[default]
    None,
    /// The straight underline.
    Straight,
    /// The curly underline.
    Curly,
}

/// The text decoration lines of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    /// The underline.
    pub underline: Underline,
    /// Whether strike.
    pub strike: bool,
}

/// One character cell with its colors and attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub(crate) text: SmolStr,
    /// The fg.
    pub fg: Ink,
    /// The bg.
    pub bg: Ink,
    /// Whether bold.
    pub bold: bool,
    /// Whether italic.
    pub italic: bool,
    /// Whether dim.
    pub dim: bool,
    /// The lines.
    pub lines: Lines,
    /// The underline ink.
    pub underline_ink: Ink,
    pub(crate) link: Option<SmolStr>,
}

impl Cell {
    /// The grapheme painted in this cell.
    #[must_use]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Sets the grapheme painted in this cell.
    pub fn set_text(&mut self, text: &str) {
        self.text = SmolStr::new(text);
    }

    /// The hyperlink this cell opens.
    #[must_use]
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    /// Returns the blank cell.
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
    pub(in crate::terminal) strike: bool,
    pub(in crate::terminal) underline: Underline,
}
