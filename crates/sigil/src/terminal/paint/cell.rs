use crate::style::{Color, Tone};

use super::glyph::{Glyph, Link};

/// The color source of a cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ink {
    /// No color is set; the terminal uses its default color.
    #[default]
    Bare,
    /// The ANSI color of a tone from the palette of the terminal; `Tone::Neutral` is the default color.
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

const BOLD: u8 = 1 << 0;
const ITALIC: u8 = 1 << 1;
const DIM: u8 = 1 << 2;
const STRIKE: u8 = 1 << 3;

/// One character cell with its colors and attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cell {
    pub(crate) text: Glyph,
    pub(crate) fg: Ink,
    pub(crate) bg: Ink,
    pub(crate) flags: u8,
    pub(crate) underline: Underline,
    pub(crate) underline_ink: Ink,
    pub(crate) link: Option<Link>,
}

impl Cell {
    const fn set_flag(&mut self, bit: u8, value: bool) {
        if value {
            self.flags |= bit;
        } else {
            self.flags &= !bit;
        }
    }

    pub(crate) const fn set_bold(&mut self, value: bool) {
        self.set_flag(BOLD, value);
    }

    pub(crate) const fn set_italic(&mut self, value: bool) {
        self.set_flag(ITALIC, value);
    }

    pub(crate) const fn set_dim(&mut self, value: bool) {
        self.set_flag(DIM, value);
    }

    pub(crate) const fn set_strike(&mut self, value: bool) {
        self.set_flag(STRIKE, value);
    }

    /// The grapheme painted in this cell.
    #[must_use]
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Sets the grapheme painted in this cell.
    pub fn set_text(&mut self, text: &str) {
        self.text = Glyph::new(text);
    }

    /// The color of the text; `Ink::Bare` by default.
    #[must_use]
    pub const fn fg(&self) -> Ink {
        self.fg
    }

    /// The color behind the text; `Ink::Bare` by default.
    #[must_use]
    pub const fn bg(&self) -> Ink {
        self.bg
    }

    /// Whether the text is bold; bold wins over `dim`.
    #[must_use]
    pub const fn bold(&self) -> bool {
        (self.flags & BOLD) != 0
    }

    /// Whether the text is italic.
    #[must_use]
    pub const fn italic(&self) -> bool {
        (self.flags & ITALIC) != 0
    }

    /// Whether the text is faint; ignored when `bold` is set.
    #[must_use]
    pub const fn dim(&self) -> bool {
        (self.flags & DIM) != 0
    }

    /// The underline under the text; `Underline::None` by default.
    #[must_use]
    pub const fn underline(&self) -> Underline {
        self.underline
    }

    /// Whether a line strikes through the text.
    #[must_use]
    pub const fn strike(&self) -> bool {
        (self.flags & STRIKE) != 0
    }

    /// The color of the underline; `Ink::Bare` uses the text color.
    #[must_use]
    pub const fn underline_ink(&self) -> Ink {
        self.underline_ink
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
            flags,
            underline,
            underline_ink,
            link,
        } = *self;
        fg == other.fg
            && bg == other.bg
            && flags == other.flags
            && underline == other.underline
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
            flags: 0,
            underline: Underline::None,
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
