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
    queue!(out, Clear(ClearType::UntilNewLine))
}

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

fn cube(colour: Rgba8) -> u8 {
    let step = |channel: u8| u16::from(channel) * 5 / 255;
    let index = 16 + 36 * step(colour.r) + 6 * step(colour.g) + step(colour.b);
    u8::try_from(index).unwrap_or(u8::MAX)
}

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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "write_tests.rs"]
mod tests;
