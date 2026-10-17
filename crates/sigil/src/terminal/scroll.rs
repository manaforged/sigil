use std::io::{self, Write};

use crossterm::style::{Attribute, SetAttribute};
use crossterm::{cursor, queue};

use crate::terminal::paint::{Cell, Grid, Ink};

const MIN_GAIN: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shift {
    top: usize,
    bottom: usize,
    by: usize,
    up: bool,
}

impl Shift {
    pub(crate) fn find(prev: &Grid, next: &Grid) -> Option<Self> {
        let rows = next.rows();
        if prev.rows() != rows
            || prev.cols() != next.cols()
            || !prev.images().is_empty()
            || !next.images().is_empty()
        {
            return None;
        }
        let sign = |grid: &Grid| -> Vec<u64> { (0..rows).map(|row| sign(grid.row(row))).collect() };
        let (old, new) = (sign(prev), sign(next));
        if old.iter().zip(&new).filter(|(was, now)| was != now).count() < MIN_GAIN {
            return None;
        }
        let mut best = None;
        let mut most = MIN_GAIN - 1;
        for by in 1..rows {
            for up in [true, false] {
                let (from, to) = if up { (by, 0) } else { (0, by) };
                let matches = |row: usize| old.get(row + from) == new.get(row + to);
                let mut row = 0;
                while row + by < rows {
                    if !matches(row) {
                        row += 1;
                        continue;
                    }
                    let top = row;
                    while row + by < rows && matches(row) {
                        row += 1;
                    }
                    let gain = (top + to..row + to)
                        .filter(|&line| old.get(line) != new.get(line))
                        .count();
                    if gain > most {
                        most = gain;
                        best = Some(Self {
                            top,
                            bottom: row + by - 1,
                            by,
                            up,
                        });
                    }
                }
            }
        }
        best.filter(|shift| shift.holds(prev, next))
    }

    fn holds(&self, prev: &Grid, next: &Grid) -> bool {
        (self.top..=self.bottom).all(|row| {
            self.source(prev, row)
                .is_none_or(|old| old == next.row(row))
        })
    }

    pub(crate) fn source<'g>(&self, prev: &'g Grid, row: usize) -> Option<&'g [Cell]> {
        if row < self.top || row > self.bottom {
            return Some(prev.row(row));
        }
        if self.up {
            (row + self.by <= self.bottom).then(|| prev.row(row + self.by))
        } else {
            (row >= self.top + self.by).then(|| prev.row(row - self.by))
        }
    }

    pub(crate) fn emit(&self, out: &mut impl Write) -> io::Result<()> {
        let clip = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
        queue!(out, SetAttribute(Attribute::Reset))?;
        write!(out, "\x1b[{};{}r", self.top + 1, self.bottom + 1)?;
        let (edge, step): (usize, &[u8]) = if self.up {
            (self.bottom, b"\x1bD")
        } else {
            (self.top, b"\x1bM")
        };
        queue!(out, cursor::MoveTo(0, clip(edge)))?;
        for _ in 0..self.by {
            out.write_all(step)?;
        }
        out.write_all(b"\x1b[r")
    }
}

fn sign(cells: &[Cell]) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    let mix = |hash: u64, value: u64| (hash ^ value).wrapping_mul(PRIME);
    cells.iter().fold(0xcbf2_9ce4_8422_2325, |hash, cell| {
        let hash = cell
            .text
            .bytes()
            .fold(hash, |hash, byte| mix(hash, u64::from(byte)));
        mix(mix(hash, ink(cell.fg)), ink(cell.bg))
    })
}

const fn ink(ink: Ink) -> u64 {
    match ink {
        Ink::Bare => 0,
        Ink::Of(tone) => 1 + tone as u64,
        Ink::Exact(colour) => {
            1 << 32
                | (colour.r as u64) << 24
                | (colour.g as u64) << 16
                | (colour.b as u64) << 8
                | colour.a as u64
        }
    }
}
