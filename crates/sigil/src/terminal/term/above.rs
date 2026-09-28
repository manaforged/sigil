use std::cell::Cell;
use std::fmt;
use std::io::{self, Write};
use std::iter::Peekable;
use std::str::Chars;
use std::sync::atomic::Ordering;

use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};
use unicode_segmentation::UnicodeSegmentation;

use super::{INLINE, LIVE, LIVE_ROWS, REDRAW, current_caps};
use crate::app::cell_width;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintAboveError {
    NotLive,
    FullScreen,
}

impl fmt::Display for PrintAboveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotLive => "print_above: no live terminal on this thread",
            Self::FullScreen => "print_above: the terminal host is not inline",
        })
    }
}

impl std::error::Error for PrintAboveError {}

impl From<PrintAboveError> for io::Error {
    fn from(error: PrintAboveError) -> Self {
        Self::new(io::ErrorKind::Unsupported, error)
    }
}

pub fn print_above(text: &str) -> io::Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    if !LIVE.with(Cell::get) {
        return Err(PrintAboveError::NotLive.into());
    }
    if !INLINE.load(Ordering::Relaxed) {
        return Err(PrintAboveError::FullScreen.into());
    }
    let cols = crossterm::terminal::size().map_or(usize::MAX, |(cols, _)| usize::from(cols));
    let mut out = io::stdout().lock();
    let sync = current_caps().sync;
    if sync {
        queue!(out, Print("\x1b[?2026h"))?;
    }
    let rows = LIVE_ROWS.with(Cell::get).max(1);
    for line in text.lines() {
        queue!(out, cursor::MoveToColumn(0))?;
        if rows > 1 {
            queue!(out, cursor::MoveUp(rows - 1))?;
        }
        let line = clip(&strip(line), cols);
        queue!(out, Clear(ClearType::CurrentLine), Print(line))?;
        queue!(out, cursor::MoveToColumn(0))?;
        if rows > 1 {
            queue!(out, cursor::MoveDown(rows - 1))?;
        }
        queue!(out, Print("\n"))?;
    }
    if sync {
        queue!(out, Print("\x1b[?2026l"))?;
    }
    out.flush()?;
    REDRAW.with(|flag| flag.set(true));
    Ok(())
}

fn strip(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => skip_escape(&mut chars),
            '\u{9b}' => skip_csi(&mut chars),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => skip_string(&mut chars),
            '\t' => out.push(' '),
            ch if ch.is_control() => {}
            ch => out.push(ch),
        }
    }
    out
}

fn skip_escape(chars: &mut Peekable<Chars<'_>>) {
    match chars.next() {
        Some('[') => skip_csi(chars),
        Some(']' | 'P' | '^' | '_' | 'X') => skip_string(chars),
        _ => {}
    }
}

fn skip_csi(chars: &mut Peekable<Chars<'_>>) {
    for ch in chars.by_ref() {
        if ('\x40'..='\x7e').contains(&ch) {
            return;
        }
    }
}

fn skip_string(chars: &mut Peekable<Chars<'_>>) {
    while let Some(ch) = chars.next() {
        match ch {
            '\x07' | '\u{9c}' => return,
            '\x1b' if chars.peek() == Some(&'\\') => {
                chars.next();
                return;
            }
            _ => {}
        }
    }
}

fn clip(line: &str, cols: usize) -> String {
    let mut out = String::with_capacity(line.len());
    let mut used = 0_usize;
    for grapheme in line.graphemes(true) {
        used = used.saturating_add(cell_width(grapheme));
        if used > cols {
            break;
        }
        out.push_str(grapheme);
    }
    out
}

#[cfg(test)]
#[path = "above_tests.rs"]
mod tests;
