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
pub(crate) enum Calibration {
    #[default]
    Unmeasured,
    #[cfg(unix)]
    Silent,
    #[cfg(unix)]
    Wrapped,
    #[cfg(unix)]
    Columns([usize; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeRow {
    Fixed,
}

thread_local! {
    static MEASURED: Cell<Calibration> = const { Cell::new(Calibration::Unmeasured) };
}

pub(crate) fn keep(measured: Calibration) {
    MEASURED.set(measured);
}

#[cfg(unix)]
pub(crate) fn query(ProbeRow::Fixed: ProbeRow) -> String {
    let mut text = String::from(PROBE_ROW);
    text.push_str(HOME);
    text.push_str(REPORT);
    for sample in SAMPLES {
        text.push_str(HOME);
        text.push_str(sample);
        text.push_str(REPORT);
    }
    text.push_str(HOME);
    text
}

#[cfg(unix)]
pub(crate) fn measure(reports: &[(u16, u16)]) -> Calibration {
    let Some(start) = reports
        .iter()
        .position(|&(row, col)| row != KEY_REPORT_ROW || col == 0)
    else {
        return Calibration::Silent;
    };
    let keys = reports
        .get(start)
        .is_some_and(|&(row, _)| row != KEY_REPORT_ROW);
    let reports: Vec<_> = reports
        .iter()
        .skip(start)
        .copied()
        .filter(|&(row, _)| !keys || row != KEY_REPORT_ROW)
        .collect();
    let Some((&(row, _), rest)) = reports.split_first() else {
        return Calibration::Silent;
    };
    if rest.len() < SAMPLES.len() {
        return Calibration::Silent;
    }
    let mut columns = [0; 4];
    for (slot, &(at, col)) in columns.iter_mut().zip(rest) {
        if at != row {
            return Calibration::Wrapped;
        }
        *slot = usize::from(col);
    }
    Calibration::Columns(columns)
}

/// Measures the width of ambiguous characters in this terminal.
pub(crate) fn calibrate(out: &mut impl Write) -> Result<(), crate::Error> {
    set_cell_width(CellWidth::Unicode);
    let [modifier, joiner, nonjoiner, keycap] = match MEASURED.get() {
        #[cfg(unix)]
        Calibration::Columns(columns) => columns,
        #[cfg(unix)]
        Calibration::Silent => {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "terminal did not report the cursor position",
            )
            .into());
        }
        #[cfg(unix)]
        Calibration::Wrapped => return Err(wrapped()),
        Calibration::Unmeasured => live(out)?,
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

fn wrapped() -> crate::Error {
    io::Error::new(io::ErrorKind::InvalidData, "terminal width probe wrapped").into()
}

fn live(out: &mut impl Write) -> Result<[usize; 4], crate::Error> {
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

fn probe(out: &mut impl Write, home: (u16, u16), text: &str) -> Result<usize, crate::Error> {
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

#[cfg(all(test, unix))]
#[path = "calibrate_tests.rs"]
mod tests;
