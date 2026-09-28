use std::io::{self, Write};

use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};

use super::{Pen, Style, same_row};
use crate::terminal::caps::{Caps, Cursor};
use crate::terminal::paint::Grid;

pub(crate) fn inline(
    out: &mut impl Write,
    prev: Option<&Grid>,
    next: &Grid,
    last_rows: usize,
    caps: &Caps,
) -> io::Result<()> {
    let rows = next.rows();
    let painted = last_rows.max(rows);
    let clip = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
    queue!(out, cursor::MoveToColumn(0))?;
    if last_rows > 1 {
        queue!(out, cursor::MoveUp(clip(last_rows - 1)))?;
    }
    let prev = prev.filter(|prev| prev.cols() == next.cols());
    let mut pen = Pen::default();
    for row in 0..painted {
        if row > 0 {
            queue!(out, Print("\r\n"))?;
        }
        if row >= rows {
            queue!(out, Clear(ClearType::CurrentLine))?;
            continue;
        }
        let end = next.painted_width(row);
        let cells = next.row(row).get(..end).unwrap_or_default();
        let same = prev
            .is_some_and(|prev| row < prev.rows() && same_row(prev.row(row), next.row(row), caps));
        if same {
            continue;
        }
        queue!(out, Clear(ClearType::CurrentLine))?;
        for cell in cells {
            pen.pick(out, Style::of(cell, caps))?;
            pen.line.push_str(&cell.text);
        }
        pen.flush(out)?;
        pen.rest(out)?;
    }
    inline_cursor(out, next.cursor(), rows, painted, caps)
}

fn inline_cursor(
    out: &mut impl Write,
    at: Option<(usize, usize)>,
    rows: usize,
    painted: usize,
    caps: &Caps,
) -> io::Result<()> {
    let Some((col, row)) = at.filter(|(_, row)| *row < rows) else {
        return queue!(out, cursor::Hide);
    };
    if caps.cursor == Cursor::Bar {
        queue!(out, cursor::SetCursorStyle::SteadyBar)?;
    }
    let rise = painted - 1 - row;
    if rise > 0 {
        queue!(out, cursor::MoveUp(u16::try_from(rise).unwrap_or(u16::MAX)))?;
    }
    queue!(
        out,
        cursor::MoveToColumn(u16::try_from(col).unwrap_or(u16::MAX)),
        cursor::Show
    )
}
