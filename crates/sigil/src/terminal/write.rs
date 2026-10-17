use std::io::{self, Write};

use crossterm::style::{
    Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
    SetUnderlineColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};

use crate::terminal::caps::{Caps, ColorDepth, Cursor, Erase};
use crate::terminal::colour::paint;
use crate::terminal::paint::{CONTINUES, Cell, Grid, Ink, Underline};
use smol_str::SmolStr;

mod diff;
mod inline;

pub(crate) use diff::{dirty, unchanged};

pub(crate) use inline::inline;

pub(crate) struct Measured<'a, W> {
    inner: &'a mut W,
    pub(crate) bytes: u64,
    pub(crate) writes: u64,
}

impl<'a, W: Write> Measured<'a, W> {
    pub(crate) const fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
            bytes: 0,
            writes: 0,
        }
    }
}

impl<W: Write> Write for Measured<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.bytes += written as u64;
        self.writes += 1;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub fn render(out: &mut impl Write, prev: &Grid, next: &Grid, depth: ColorDepth) -> io::Result<()> {
    let caps = Caps {
        color: depth,
        ..Caps::default()
    };
    if prev.cols() == next.cols() && prev.rows() == next.rows() {
        dirty(out, prev, next, &caps, None)
    } else {
        all(out, next, &caps)
    }
}

pub(crate) fn all(out: &mut impl Write, grid: &Grid, caps: &Caps) -> io::Result<()> {
    let mut pen = Pen::default();
    for row in 0..grid.rows() {
        row_bytes(out, grid, row, &mut pen, caps)?;
    }
    pen.rest(out)
}

pub(crate) fn same_row(prev: &[Cell], next: &[Cell], caps: &Caps) -> bool {
    prev.len() == next.len()
        && prev
            .iter()
            .zip(next)
            .all(|(old, new)| same_cell(old, new, caps))
}

fn same_cell(old: &Cell, new: &Cell, caps: &Caps) -> bool {
    old == new || (old.text == new.text && Style::of(old, caps) == Style::of(new, caps))
}

pub(crate) fn cursor(
    out: &mut impl Write,
    at: Option<(usize, usize)>,
    caret: bool,
    caps: &Caps,
) -> io::Result<()> {
    let Some((col, row)) = at else {
        return queue!(out, cursor::Hide);
    };
    if !caret {
        return queue!(out, cursor::Hide, cursor::MoveTo(clip(col), clip(row)));
    }
    if caps.cursor == Cursor::Bar {
        queue!(out, cursor::SetCursorStyle::SteadyBar)?;
    }
    queue!(out, cursor::MoveTo(clip(col), clip(row)), cursor::Show)
}

#[derive(Clone, PartialEq, Eq)]
struct Style {
    fg: Color,
    bg: Color,
    intensity: Attribute,
    italic: bool,
    strike: bool,
    underline: Underline,
    underline_color: Option<Color>,
    link: Option<SmolStr>,
}

impl Style {
    fn of(cell: &Cell, caps: &Caps) -> Self {
        let (foreground, background) = terminal_inks(cell);
        let underline = match cell.lines.underline {
            Underline::Curly if !caps.underline_styles => Underline::Straight,
            other => other,
        };
        let styled = caps.underline_styles && underline != Underline::None;
        Self {
            fg: paint(foreground, caps.color),
            bg: paint(background, caps.color),
            intensity: if cell.bold {
                Attribute::Bold
            } else if cell.dim {
                Attribute::Dim
            } else {
                Attribute::NormalIntensity
            },
            italic: cell.italic,
            strike: cell.lines.strike,
            underline,
            underline_color: (styled && cell.underline_ink.set())
                .then(|| paint(cell.underline_ink, caps.color))
                .filter(|color| *color != Color::Reset),
            link: cell
                .link
                .clone()
                .filter(|url| caps.hyperlinks && !url.chars().any(char::is_control)),
        }
    }

    const fn plain() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            intensity: Attribute::NormalIntensity,
            italic: false,
            strike: false,
            underline: Underline::None,
            underline_color: None,
            link: None,
        }
    }
}

#[derive(Default)]
struct Pen {
    line: String,
    style: Option<Style>,
}

impl Pen {
    fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.line.is_empty() {
            queue!(out, Print(&*self.line))?;
            self.line.clear();
        }
        Ok(())
    }

    fn pick(&mut self, out: &mut impl Write, next: Style) -> io::Result<()> {
        if self.style.as_ref() != Some(&next) {
            self.flush(out)?;
            switch(out, self.style.as_ref(), &next)?;
            self.style = Some(next);
        }
        Ok(())
    }

    fn rest(&mut self, out: &mut impl Write) -> io::Result<()> {
        let Some(style) = self.style.take_if(|style| *style != Style::plain()) else {
            return Ok(());
        };
        if style.link.is_some() {
            queue!(out, Print(CLOSE_LINK))?;
        }
        self.style = Some(Style::plain());
        queue!(out, SetAttribute(Attribute::Reset))
    }
}

fn row_bytes(
    out: &mut impl Write,
    grid: &Grid,
    row: usize,
    pen: &mut Pen,
    caps: &Caps,
) -> io::Result<()> {
    let end = grid.painted_width(row);
    cells(out, grid.row(row), (row, 0, end, true), pen, caps)
}

fn cells(
    out: &mut impl Write,
    row_cells: &[Cell],
    (row, start, end, tail): (usize, usize, usize, bool),
    pen: &mut Pen,
    caps: &Caps,
) -> io::Result<()> {
    queue!(out, cursor::MoveTo(clip(start), clip(row)))?;
    let fill = if tail {
        filled(row_cells, (start, end), caps)
    } else {
        None
    };
    let stop = fill.as_ref().map_or(end, |(from, _)| *from);
    for cell in row_cells.iter().take(stop).skip(start) {
        pen.pick(out, Style::of(cell, caps))?;
        pen.line.push_str(&cell.text);
    }
    pen.flush(out)?;
    if let Some((_, paint)) = fill {
        pen.pick(out, paint)?;
        queue!(out, Clear(ClearType::UntilNewLine))?;
    } else if tail && stop < row_cells.len() {
        pen.rest(out)?;
        queue!(out, Clear(ClearType::UntilNewLine))?;
    }
    if pen.style.as_ref().is_some_and(|style| style.link.is_some()) {
        pen.rest(out)?;
    }
    Ok(())
}

fn filled(row_cells: &[Cell], (start, end): (usize, usize), caps: &Caps) -> Option<(usize, Style)> {
    let span = row_cells.get(start..end)?;
    let paint = Style::of(span.last()?, caps);
    if caps.erase == Erase::Default && paint.bg != Color::Reset {
        return None;
    }
    let reaches = end == row_cells.len() || paint.bg == Color::Reset;
    if !reaches || paint.underline != Underline::None || paint.strike || paint.link.is_some() {
        return None;
    }
    let blank = |cell: &Cell| cell.text == " " && Style::of(cell, caps) == paint;
    let from = span
        .iter()
        .rposition(|cell| !blank(cell))
        .map_or(start, |index| start + index + 1);
    (end - from >= FILL_MIN).then_some((from, paint))
}

const FILL_MIN: usize = 4;

const CLOSE_LINK: &str = "\x1b]8;;\x1b\\";

fn switch(out: &mut impl Write, prev: Option<&Style>, next: &Style) -> io::Result<()> {
    if prev.is_none() {
        queue!(out, SetAttribute(Attribute::Reset))?;
    }
    let prev = prev.cloned().unwrap_or_else(Style::plain);
    if prev.intensity != next.intensity {
        if prev.intensity != Attribute::NormalIntensity {
            queue!(out, SetAttribute(Attribute::NormalIntensity))?;
        }
        if next.intensity != Attribute::NormalIntensity {
            queue!(out, SetAttribute(next.intensity))?;
        }
    }
    if prev.italic != next.italic {
        queue!(
            out,
            SetAttribute(flag(next.italic, Attribute::Italic, Attribute::NoItalic))
        )?;
    }
    if prev.fg != next.fg {
        queue!(out, SetForegroundColor(next.fg))?;
    }
    if prev.bg != next.bg {
        queue!(out, SetBackgroundColor(next.bg))?;
    }
    lines(out, &prev, next)
}

fn lines(out: &mut impl Write, prev: &Style, next: &Style) -> io::Result<()> {
    if prev.strike != next.strike {
        queue!(
            out,
            SetAttribute(flag(
                next.strike,
                Attribute::CrossedOut,
                Attribute::NotCrossedOut
            ))
        )?;
    }
    if prev.underline != next.underline {
        queue!(
            out,
            SetAttribute(match next.underline {
                Underline::None => Attribute::NoUnderline,
                Underline::Straight => Attribute::Underlined,
                Underline::Curly => Attribute::Undercurled,
            })
        )?;
    }
    if prev.underline_color != next.underline_color {
        queue!(
            out,
            SetUnderlineColor(next.underline_color.unwrap_or(Color::Reset))
        )?;
    }
    if prev.link != next.link {
        match &next.link {
            Some(url) => queue!(out, Print(format!("\x1b]8;;{url}\x1b\\")))?,
            None => queue!(out, Print(CLOSE_LINK))?,
        }
    }
    Ok(())
}

const fn flag(on: bool, set: Attribute, unset: Attribute) -> Attribute {
    if on { set } else { unset }
}

fn clip(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn terminal_inks(cell: &Cell) -> (Ink, Ink) {
    (cell.fg, if cell.text == "█" { cell.fg } else { cell.bg })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "write_tests.rs"]
mod tests;
