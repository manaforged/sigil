use std::io::{self, Write};
use std::sync::OnceLock;

use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};
use sigil::style::Tone;

use crate::paint::{Cell, Grid, Ink};
use sigil::style::Rgba8;

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
    let mut style: Option<(Ink, Ink, bool, bool)> = None;
    let end = grid.painted_width(row);
    for cell in grid.row(row).iter().take(end) {
        let (foreground, background) = terminal_inks(cell);
        let next = (foreground, background, cell.bold, cell.italic);
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
                SetAttribute(if cell.italic {
                    Attribute::Italic
                } else {
                    Attribute::NoItalic
                }),
                SetForegroundColor(paint(foreground)),
                SetBackgroundColor(paint(background)),
            )?;
            style = Some(next);
        }
        line.push_str(&cell.text);
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

fn terminal_inks(cell: &Cell) -> (Ink, Ink) {
    (cell.fg, if cell.text == "█" { cell.fg } else { cell.bg })
}

fn truecolor() -> bool {
    static ANSWER: OnceLock<bool> = OnceLock::new();
    *ANSWER.get_or_init(|| {
        let program = std::env::var("TERM_PROGRAM").ok();
        let color = std::env::var("COLORTERM").ok();
        supports_truecolor(program.as_deref(), color.as_deref())
    })
}

fn supports_truecolor(_program: Option<&str>, color: Option<&str>) -> bool {
    color.is_some_and(|value| value.contains("truecolor") || value.contains("24bit"))
}

pub(crate) fn cube(colour: Rgba8) -> u8 {
    let (red, r) = nearest_level(colour.r);
    let (green, g) = nearest_level(colour.g);
    let (blue, b) = nearest_level(colour.b);
    let cube_distance = distance(colour, red, green, blue);
    let mean = (u16::from(colour.r) + u16::from(colour.g) + u16::from(colour.b)) / 3;
    let gray_index = mean.saturating_sub(8).saturating_add(5) / 10;
    let gray_index = gray_index.min(23) as u8;
    let gray = 8 + gray_index * 10;
    if distance(colour, gray, gray, gray) < cube_distance {
        232 + gray_index
    } else {
        16 + 36 * r + 6 * g + b
    }
}

fn nearest_level(channel: u8) -> (u8, u8) {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let mut best = (LEVELS[0], 0_u8);
    let mut delta = channel.abs_diff(LEVELS[0]);
    for (index, level) in (1_u8..).zip(LEVELS.into_iter().skip(1)) {
        if channel.abs_diff(level) < delta {
            best = (level, index);
            delta = channel.abs_diff(level);
        }
    }
    best
}

fn distance(colour: Rgba8, red: u8, green: u8, blue: u8) -> u32 {
    let square = |value: u8, target: u8| u32::from(value.abs_diff(target)).pow(2);
    square(colour.r, red) + square(colour.g, green) + square(colour.b, blue)
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
