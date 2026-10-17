use std::time::{Duration, Instant};

use crate::style::Tone;
use crate::terminal::paint::{Cell, Grid, Ink};
use crate::terminal::transcript::{Reading, Spot};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusCursor {
    #[default]
    Follow,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Access {
    pub focus_cursor: FocusCursor,
    pub status: Duration,
}

impl Default for Access {
    fn default() -> Self {
        Self {
            focus_cursor: FocusCursor::Follow,
            status: Duration::from_secs(5),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Speech {
    #[default]
    StatusRow,
    Above,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    Keep,
    Move(Option<(usize, usize)>),
}

#[derive(Default)]
pub(crate) struct Announcer {
    pub(crate) caret: bool,
    pub(crate) access: Access,
    pub(crate) speech: Speech,
    status: Option<(String, Instant)>,
    above: Vec<String>,
    under: Option<(usize, Vec<Cell>)>,
}

impl Announcer {
    pub(crate) fn hear(&mut self, reading: &Reading) -> bool {
        let mut changed = false;
        for message in &reading.spoken {
            match self.speech {
                Speech::Above => self.above.push(message.clone()),
                Speech::StatusRow => {
                    self.status = Some((message.clone(), Instant::now() + self.access.status));
                    changed = true;
                }
            }
        }
        if self.deadline().is_some_and(|due| due <= Instant::now()) {
            self.status = None;
            changed = true;
        }
        changed
    }

    pub(crate) fn deadline(&self) -> Option<Instant> {
        self.status.as_ref().map(|(_, due)| *due)
    }

    pub(crate) fn take_above(&mut self) -> Vec<String> {
        core::mem::take(&mut self.above)
    }

    pub(crate) fn focus_cell(&self, grid: &Grid, reading: &Reading) -> Place {
        if self.caret || self.access.focus_cursor == FocusCursor::Off {
            return Place::Keep;
        }
        Place::Move(reading.focus.as_ref().and_then(|spot| locate(grid, spot)))
    }

    pub(crate) fn restore(&mut self, grid: &mut Grid) {
        if let Some((row, cells)) = self.under.take() {
            grid.set_row(row, &cells);
        }
    }

    pub(crate) fn apply(&mut self, grid: &mut Grid, raw: bool, focus: Place) {
        if raw {
            self.under = None;
        } else {
            self.restore(grid);
        }
        if let Place::Move(cell) = focus {
            grid.place_cursor(cell);
        }
        let Some((message, _)) = &self.status else {
            return;
        };
        let cols = grid.cols();
        let row = grid.rows().saturating_sub(1);
        self.under = Some((row, grid.row(row).to_vec()));
        grid.set_row(row, &vec![Cell::blank(); cols]);
        grid.overlay_text(0, row, message, Ink::Of(Tone::Neutral));
    }
}

fn locate(grid: &Grid, spot: &Spot) -> Option<(usize, usize)> {
    if spot.from.0 >= grid.cols() || spot.from.1 >= grid.rows() {
        return None;
    }
    let rows = spot.from.1..spot.to.1.min(grid.rows());
    let end = spot.to.0.min(grid.cols());
    let cells = || {
        rows.clone()
            .flat_map(move |row| (spot.from.0..end).map(move |col| (col, row)))
    };
    let named = || {
        cells().find(|&(col, row)| {
            let tail: String = grid
                .row(row)
                .get(col..end)
                .unwrap_or_default()
                .iter()
                .map(|cell| cell.text.as_str())
                .collect();
            tail.starts_with(&spot.name)
        })
    };
    let readable = || {
        cells().find(|&(col, row)| {
            grid.row(row)
                .get(col)
                .is_some_and(|cell| readable(&cell.text))
        })
    };
    let named = if spot.name.is_empty() { None } else { named() };
    named.or_else(readable).or(Some(spot.from))
}

fn readable(text: &str) -> bool {
    text.chars().any(|ch| {
        !ch.is_whitespace()
            && !('\u{2500}'..='\u{259f}').contains(&ch)
            && !('\u{1fb00}'..='\u{1fbff}').contains(&ch)
    })
}
