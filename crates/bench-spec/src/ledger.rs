//! The Ledger scenario: one fixed script every framework runner replays.
//!
//! A pulse is one 16 ms driver fire. Every pulse ticks the price stream;
//! some pulses also apply one [`Step`]. [`Tally::expect`] is the state a
//! correct run ends in; a record whose checks fail is not comparable.

use std::time::Duration;

use data_table::ledger::{Row, TICK_STRIDE, tick};
use data_table::{FieldKind, TABLE_WIDTH, TOTAL_ITEMS, VIEWPORT_HEIGHT};

use crate::Check;

pub const SCENARIO: &str = "ledger-v1";
pub const ROWS: usize = TOTAL_ITEMS;
pub const PULSE: Duration = Duration::from_millis(16);
pub const PULSES: u32 = 900;
pub const ROW_H: f32 = 24.0;
pub const WINDOW: (u32, u32) = (1280, 1000);
pub const WINDOW_PX: (f32, f32) = (1280.0, 1000.0);
pub const VIEWPORT: (f32, f32) = (TABLE_WIDTH, VIEWPORT_HEIGHT);
pub const FILTER: &str = "AB";
pub const FILTER_SAMPLE: u32 = 600;
pub const EDITS: u32 = 5;

const SLOW_SCROLL: (u32, u32, u32) = (0, 300, 2);
const FAST_SCROLL: (u32, u32, u32) = (400, 550, 8);
const EDIT_START: u32 = 660;
const EDIT_EVERY: u32 = 40;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Scroll(u32),
    Sort(FieldKind),
    Filter(&'static str),
    Select(u32),
    Edit,
    Commit(f64),
    Undo,
    Theme,
}

#[must_use]
pub fn step(pulse: u32) -> Option<Step> {
    if let Some(rows) = scroll(pulse, SLOW_SCROLL).or_else(|| scroll(pulse, FAST_SCROLL)) {
        return Some(Step::Scroll(rows));
    }
    match pulse {
        300 => return Some(Step::Sort(FieldKind::Symbol)),
        360 => return Some(Step::Sort(FieldKind::LastDone)),
        560 => return Some(Step::Filter(FILTER)),
        640 => return Some(Step::Filter("")),
        860 | 890 => return Some(Step::Theme),
        _ => {}
    }
    edit(pulse)
}

fn scroll(pulse: u32, (from, to, rows): (u32, u32, u32)) -> Option<u32> {
    (from..to).contains(&pulse).then_some(rows)
}

fn edit(pulse: u32) -> Option<Step> {
    let since = pulse.checked_sub(EDIT_START)?;
    let (cycle, offset) = (since / EDIT_EVERY, since % EDIT_EVERY);
    if cycle >= EDITS {
        return None;
    }
    match offset {
        0 => Some(Step::Select(cycle * 100 + 7)),
        1 => Some(Step::Edit),
        10 => Some(Step::Commit(150.0 + f64::from(cycle))),
        25 => Some(Step::Undo),
        _ => None,
    }
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "row offsets stay far below f32 integer precision"
)]
pub fn scroll_px(rows: u32) -> f32 {
    (f64::from(rows) * f64::from(ROW_H)) as f32
}

#[must_use]
pub fn scroll_rows() -> u32 {
    [SLOW_SCROLL, FAST_SCROLL]
        .iter()
        .map(|(from, to, rows)| (to - from) * rows)
        .sum()
}

#[must_use]
pub fn filter_hits(book: &[Row]) -> usize {
    book.iter().filter(|row| row.matches(FILTER)).count()
}

#[must_use]
pub fn moved_total(rows: usize, pulses: u32) -> u64 {
    let mut prices = vec![1.0; rows];
    (0..u64::from(pulses))
        .map(|step| tick(&mut prices, step) as u64)
        .sum()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub pulses: u32,
    pub ticks: u64,
    pub moved: u64,
    pub edits: u32,
    pub undo_depth: usize,
    pub visible: usize,
    pub filter_hits: usize,
    pub scroll_rows: u32,
    pub stride: usize,
}

impl Tally {
    #[must_use]
    pub fn expect(book: &[Row]) -> Self {
        Self {
            pulses: PULSES,
            ticks: u64::from(PULSES),
            moved: moved_total(book.len(), PULSES),
            edits: EDITS,
            undo_depth: 0,
            visible: book.len(),
            filter_hits: filter_hits(book),
            scroll_rows: scroll_rows(),
            stride: TICK_STRIDE,
        }
    }

    #[must_use]
    pub fn checks(&self, observed: &Self) -> Vec<Check> {
        let mut out = Vec::new();
        let mut check = |name: &str, want: String, got: String| {
            out.push(Check::new(
                name,
                want == got,
                format!("want {want}, got {got}"),
            ));
        };
        check(
            "pulses",
            self.pulses.to_string(),
            observed.pulses.to_string(),
        );
        check("ticks", self.ticks.to_string(), observed.ticks.to_string());
        check("moved", self.moved.to_string(), observed.moved.to_string());
        check("edits", self.edits.to_string(), observed.edits.to_string());
        check(
            "undo_depth",
            self.undo_depth.to_string(),
            observed.undo_depth.to_string(),
        );
        check(
            "visible",
            self.visible.to_string(),
            observed.visible.to_string(),
        );
        check(
            "filter_hits",
            self.filter_hits.to_string(),
            observed.filter_hits.to_string(),
        );
        check(
            "scroll_rows",
            self.scroll_rows.to_string(),
            observed.scroll_rows.to_string(),
        );
        check(
            "stride",
            self.stride.to_string(),
            observed.stride.to_string(),
        );
        out
    }
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
