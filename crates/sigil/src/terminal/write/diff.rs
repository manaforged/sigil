use std::io::{self, Write};

use super::{CONTINUES, Pen, Style, cells, row_bytes, same_cell, same_row};
use crate::terminal::caps::Caps;
use crate::terminal::paint::{Cell, Grid};
use crate::terminal::scroll::Shift;

pub(crate) fn dirty(out: &mut impl Write, prev: &Grid, next: &Grid, caps: &Caps) -> io::Result<()> {
    let shift = Shift::find(prev, next);
    let mut pen = Pen::default();
    if let Some(shift) = shift {
        shift.emit(out)?;
        pen.style = Some(Style::plain());
    }
    for row in 0..next.rows() {
        let old = shift.map_or_else(|| Some(prev.row(row)), |shift| shift.source(prev, row));
        match old {
            Some(old) if same_row(old, next.row(row), caps) => {}
            Some(old) => span(out, (old, row), next, &mut pen, caps)?,
            None => row_bytes(out, next, row, &mut pen, caps)?,
        }
    }
    pen.rest(out)
}

fn span(
    out: &mut impl Write,
    (old, row): (&[Cell], usize),
    next: &Grid,
    pen: &mut Pen,
    caps: &Caps,
) -> io::Result<()> {
    let new = next.row(row);
    let differs = |index: usize| match (old.get(index), new.get(index)) {
        (Some(was), Some(now)) => !same_cell(was, now, caps),
        _ => false,
    };
    let joined = |index: usize| {
        old.get(index).is_some_and(|cell| cell.text == CONTINUES)
            || new.get(index).is_some_and(|cell| cell.text == CONTINUES)
    };
    let width = next.painted_width(row);
    let mut from = 0;
    while let Some(first) = (from..new.len()).find(|&index| differs(index)) {
        let mut last = first;
        let mut probe = first + 1;
        while probe < new.len() && probe <= last + SKIP {
            if differs(probe) {
                last = probe;
            }
            probe += 1;
        }
        let (mut start, mut end) = (first, last + 1);
        while start > from && joined(start) {
            start -= 1;
        }
        while joined(end) {
            end += 1;
        }
        let tail = end >= width;
        cells(
            out,
            new,
            (row, start, if tail { width } else { end }, tail),
            pen,
            caps,
        )?;
        if tail {
            break;
        }
        from = end;
    }
    Ok(())
}

const SKIP: usize = 8;
