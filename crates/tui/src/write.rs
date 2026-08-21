//! Turn a cell grid into terminal bytes.
//!
//! One SGR pair covers a run of like cells, and a row is written only to its
//! last painted column: the rest is one clear. Colour goes out as 24-bit when
//! the style resolved one and falls back to the terminal's own palette when
//! it did not.

use std::io::{self, Write};
use std::sync::OnceLock;

use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};
use sigil_style::Tone;

use crate::paint::{CONTINUES, Grid, Ink};
use sigil_style::Rgba8;

/// Write the cells that differ between `prev` and `next` to `out`.
///
/// The live host calls this each frame. A host running its own terminal loop
/// over [`step`] uses it to put a frame on screen.
///
/// # Errors
///
/// Returns any write error from `out`.
pub fn render(out: &mut impl Write, prev: &Grid, next: &Grid) -> io::Result<()> {
    if prev.cols() == next.cols() && prev.rows() == next.rows() {
        dirty(out, prev, next)
    } else {
        all(out, next)
    }
}

pub(crate) fn all(out: &mut impl Write, grid: &Grid) -> io::Result<()> {
    let mut line = String::new();
    for row in 0..grid.rows() {
        row_bytes(out, grid, row, &mut line)?;
    }
    Ok(())
}

pub(crate) fn dirty(out: &mut impl Write, prev: &Grid, next: &Grid) -> io::Result<()> {
    let mut line = String::new();
    for row in 0..next.rows() {
        if next.row(row) != prev.row(row) {
            row_bytes(out, next, row, &mut line)?;
        }
    }
    Ok(())
}

fn row_bytes(out: &mut impl Write, grid: &Grid, row: usize, line: &mut String) -> io::Result<()> {
    let y = u16::try_from(row).unwrap_or(u16::MAX);
    queue!(out, cursor::MoveTo(0, y))?;
    // One SGR pair per run, not per cell: a row of one colour costs two
    // escape sequences however wide the terminal is.
    let mut style: Option<(Ink, Ink, bool)> = None;
    let end = grid.painted_width(row);
    for col in 0..end {
        let cell = grid.get(col, row).unwrap_or_default();
        let next = (cell.fg, cell.bg, cell.bold);
        if style != Some(next) {
            if !line.is_empty() {
                queue!(out, Print(&*line))?;
                line.clear();
            }
            queue!(
                out,
                SetAttribute(if cell.bold {
                    Attribute::Bold
                } else {
                    Attribute::NormalIntensity
                }),
                SetForegroundColor(paint(cell.fg)),
                SetBackgroundColor(paint(cell.bg)),
            )?;
            style = Some(next);
        }
        if cell.ch != CONTINUES {
            line.push(cell.ch);
        }
    }
    if !line.is_empty() {
        queue!(out, Print(&*line))?;
        line.clear();
    }
    if style.is_some() {
        queue!(out, SetAttribute(Attribute::Reset), ResetColor)?;
    }
    // The row was painted only up to `end`; clear whatever the last frame
    // left to the right of it rather than writing a run of blanks.
    queue!(out, Clear(ClearType::UntilNewLine))
}

/// Whether this terminal accepts 24-bit colour.
///
/// Read once. A terminal without it renders a truecolor escape as garbage or
/// ignores it, so a resolved colour has to come down to the 256-colour cube
/// instead. `COLORTERM` is what terminals actually advertise it through.
fn truecolor() -> bool {
    static ANSWER: OnceLock<bool> = OnceLock::new();
    *ANSWER.get_or_init(|| {
        let program = std::env::var("TERM_PROGRAM").ok();
        let color = std::env::var("COLORTERM").ok();
        supports_truecolor(program.as_deref(), color.as_deref())
    })
}

fn supports_truecolor(program: Option<&str>, color: Option<&str>) -> bool {
    program != Some("Apple_Terminal")
        && color.is_some_and(|value| value.contains("truecolor") || value.contains("24bit"))
}

/// Nearest colour in the 6x6x6 cube, for terminals without 24-bit colour.
fn cube(colour: Rgba8) -> u8 {
    let step = |channel: u8| u16::from(channel) * 5 / 255;
    let index = 16 + 36 * step(colour.r) + 6 * step(colour.g) + step(colour.b);
    u8::try_from(index).unwrap_or(u8::MAX)
}

/// Terminal colour for one ink.
///
/// A resolved colour goes out as 24-bit where the terminal takes it, so a
/// badge is the theme's own swatch. A bare tone falls back to the terminal's
/// palette, which is what an unthemed widget and a user's own scheme want.
fn paint(ink: Ink) -> Color {
    paint_for(ink, truecolor())
}

fn paint_for(ink: Ink, truecolor: bool) -> Color {
    match ink {
        Ink::Bare | Ink::Of(Tone::Neutral) => Color::Reset,
        Ink::Of(Tone::Primary) => Color::Blue,
        Ink::Of(Tone::Info) => Color::Cyan,
        Ink::Of(Tone::Success) => Color::Green,
        Ink::Of(Tone::Warn) => Color::Yellow,
        Ink::Of(Tone::Error) => Color::Red,
        Ink::Exact(colour) if truecolor => Color::Rgb {
            r: colour.r,
            g: colour.g,
            b: colour.b,
        },
        Ink::Exact(colour) => Color::AnsiValue(cube(colour)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "write_tests.rs"]
mod tests;
