use crate::style::{Color, Tone};

use super::glyph::{Glyph, Link};

/// The color source of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ink {
    /// No color is set; the terminal uses its default color.
    #[default]
    Bare,
    /// The ANSI color of a tone, which the palette of the terminal decides.
    Of(Tone),
    /// This RGB color, mapped to the nearest color the terminal shows.
    Exact(Color),
}

impl Ink {
    pub(super) const fn pick(colour: Color, tone: Option<Tone>) -> Self {
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
#[non_exhaustive]
pub enum Underline {
    /// No underline.
    #[default]
    None,
    /// A straight line under the text.
    Straight,
    /// A wavy line under the text, drawn straight where the terminal has no underline styles.
    Curly,
}

/// The text decoration lines of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Lines {
    /// The underline under the text; none by default.
    pub underline: Underline,
    /// Whether a line strikes through the text.
    pub strike: bool,
}

/// One character cell with its colors and attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cell {
    pub(crate) text: Glyph,
    /// The color of the text; `Ink::Bare` by default.
    pub fg: Ink,
    /// The color behind the text; `Ink::Bare` by default.
    pub bg: Ink,
    /// Whether the text is bold; bold wins over `dim`.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
    /// Whether the text is faint; ignored when `bold` is set.
    pub dim: bool,
    /// The underline and the strike-through of the text.
    pub lines: Lines,
    /// The color of the underline; `Ink::Bare` uses the text color.
    pub underline_ink: Ink,
    pub(crate) link: Option<Link>,
}

impl Cell {
    /// The grapheme painted in this cell.
    #[must_use]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Sets the grapheme painted in this cell.
    pub fn set_text(&mut self, text: &str) {
        self.text = Glyph::new(text);
    }

    /// Returns the URL that this cell opens as a hyperlink, or `None` when it has no link.
    #[must_use]
    pub fn link(&self) -> Option<&str> {
        self.link.map(Link::as_str)
    }

    pub(crate) fn styled_like(&self, other: &Self) -> bool {
        let Self {
            text: _,
            fg,
            bg,
            bold,
            italic,
            dim,
            lines,
            underline_ink,
            link,
        } = *self;
        fg == other.fg
            && bg == other.bg
            && bold == other.bold
            && italic == other.italic
            && dim == other.dim
            && lines == other.lines
            && underline_ink == other.underline_ink
            && link == other.link
    }

    /// A cell that holds one space with no colors and no attributes.
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            text: Glyph::inline(" "),
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
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent text attribute that combines freely with the others"
)]
pub(in crate::terminal) struct Face {
    pub(in crate::terminal) bold: bool,
    pub(in crate::terminal) italic: bool,
    pub(in crate::terminal) dim: bool,
    pub(in crate::terminal) strike: bool,
    pub(in crate::terminal) underline: Underline,
}
