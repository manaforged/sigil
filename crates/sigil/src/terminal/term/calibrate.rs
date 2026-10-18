use std::cell::Cell;
use std::io::{self, Write};

use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, execute};
use unicode_width::UnicodeWidthStr;

use crate::app::{CellWidth, set_cell_width};

const MODIFIER: &str = "👍🏽";
const SAMPLES: [&str; 4] = [MODIFIER, "\u{200d}", "\u{200c}", "1\u{fe0f}\u{20e3}"];
#[cfg(unix)]
const HOME: &str = "\r\x1b[2K";
#[cfg(unix)]
const REPORT: &str = "\x1b[6n";
#[cfg(unix)]
const PROBE_ROW: &str = "\x1b[2H";
#[cfg(unix)]
const KEY_REPORT_ROW: u16 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Measured {
    #[default]
    Unmeasured,
    #[cfg(unix)]
    Silent,
    #[cfg(unix)]
    Wrapped,
    #[cfg(unix)]
    Columns([usize; 4]),
}

thread_local! {
    static MEASURED: Cell<Measured> = const { Cell::new(Measured::Unmeasured) };
}

pub(crate) fn keep(measured: Measured) {
    MEASURED.set(measured);
}

#[cfg(unix)]
pub(crate) fn query() -> String {
    let mut text = format!("{PROBE_ROW}{HOME}{REPORT}");
    for sample in SAMPLES {
        text.push_str(HOME);
        text.push_str(sample);
        text.push_str(REPORT);
    }
    text.push_str(HOME);
    text
}

#[cfg(unix)]
pub(crate) fn measure(reports: &[(u16, u16)]) -> Measured {
    let reports: Vec<_> = reports
        .iter()
        .copied()
        .filter(|&(row, _)| row != KEY_REPORT_ROW)
        .collect();
    let Some((&(row, _), rest)) = reports.split_first() else {
        return Measured::Silent;
    };
    if rest.len() < SAMPLES.len() {
        return Measured::Silent;
    }
    let mut columns = [0; 4];
    for (slot, &(at, col)) in columns.iter_mut().zip(rest) {
        if at != row {
            return Measured::Wrapped;
        }
        *slot = usize::from(col);
    }
    Measured::Columns(columns)
}

pub fn calibrate(out: &mut impl Write) -> io::Result<()> {
    set_cell_width(CellWidth::Unicode);
    let [modifier, joiner, nonjoiner, keycap] = match MEASURED.get() {
        #[cfg(unix)]
        Measured::Columns(columns) => columns,
        #[cfg(unix)]
        Measured::Silent => {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "terminal did not report the cursor position",
            ));
        }
        #[cfg(unix)]
        Measured::Wrapped => return Err(wrapped()),
        Measured::Unmeasured => live(out)?,
    };
    if modifier > UnicodeWidthStr::width(MODIFIER) {
        set_cell_width(CellWidth::Scalar {
            joiner,
            nonjoiner,
            keycap: keycap.saturating_sub(1),
        });
    }
    Ok(())
}

fn wrapped() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "terminal width probe wrapped")
}

fn live(out: &mut impl Write) -> io::Result<[usize; 4]> {
    let home = cursor::position()?;
    let mut columns = [0; 4];
    for (slot, sample) in columns.iter_mut().zip(SAMPLES) {
        *slot = probe(out, home, sample)?;
    }
    execute!(
        out,
        cursor::MoveTo(0, home.1),
        Clear(ClearType::CurrentLine)
    )?;
    Ok(columns)
}

fn probe(out: &mut impl Write, home: (u16, u16), text: &str) -> io::Result<usize> {
    execute!(
        out,
        cursor::MoveTo(0, home.1),
        Clear(ClearType::CurrentLine),
        Print(text)
    )?;
    let (col, row) = cursor::position()?;
    if row != home.1 {
        return Err(wrapped());
    }
    Ok(usize::from(col))
}
