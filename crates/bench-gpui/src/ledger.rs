use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Instant;

use bench_spec::Record;
use bench_spec::ledger::{
    FILTER_SAMPLE, PULSE, PULSES, SCENARIO, Step, Tally, VIEWPORT, scroll_px, step,
};
use data_table::FieldKind;
use data_table::ledger::{
    Row, TICK_STRIDE, UNDO_DEPTH, compare, generate, parse_price, prices, tick,
};
use gpui::{Context, Timer, UniformListScrollHandle, point, px};

pub const GPUI_VERSION: &str = "0.2.2";

pub struct Ledger {
    pub book: Vec<Row>,
    pub prices: Vec<f64>,
    pub order: Vec<u32>,
    pub sort: Option<(FieldKind, bool)>,
    pub filter: String,
    pub selected: Option<u32>,
    pub editing: Option<u32>,
    pub draft: String,
    pub undo: Vec<(u32, f64)>,
    pub dark: bool,
    pub ticks: u64,
    pub moved: u64,
    pub scroll: UniformListScrollHandle,
    drive: Drive,
}

struct Drive {
    out: PathBuf,
    pulse: u32,
    marks: Vec<Instant>,
    edits: u32,
    filter_hits: usize,
    scroll_rows: u32,
}

impl Ledger {
    pub fn new(rows: usize, out: PathBuf, cx: &mut Context<'_, Self>) -> Self {
        cx.spawn(async move |this, app| {
            loop {
                Timer::after(PULSE).await;
                if !matches!(this.update(app, Self::pulse), Ok(true)) {
                    break;
                }
            }
        })
        .detach();
        Self::model(rows, out)
    }

    #[must_use]
    pub fn model(rows: usize, out: PathBuf) -> Self {
        let book = generate(rows);
        let prices = prices(&book);
        let order = (0..u32::try_from(rows).unwrap_or(u32::MAX)).collect();
        Self {
            book,
            prices,
            order,
            sort: None,
            filter: String::new(),
            selected: None,
            editing: None,
            draft: String::new(),
            undo: Vec::new(),
            dark: false,
            ticks: 0,
            moved: 0,
            scroll: UniformListScrollHandle::new(),
            drive: Drive {
                out,
                pulse: 0,
                marks: Vec::with_capacity(PULSES as usize),
                edits: 0,
                filter_hits: 0,
                scroll_rows: 0,
            },
        }
    }

    pub fn pulse(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.advance() {
            return false;
        }
        cx.notify();
        if self.drive.pulse == PULSES {
            let record = self.record();
            if let Err(err) = record.write(&self.drive.out) {
                eprintln!("ledger-gpui: write {}: {err}", self.drive.out.display());
            }
            eprintln!("ledger-gpui: {}", summary(&record));
            cx.quit();
            return false;
        }
        true
    }

    pub fn advance(&mut self) -> bool {
        if self.drive.pulse >= PULSES {
            return false;
        }
        self.drive.marks.push(Instant::now());
        self.tick();
        if let Some(step) = step(self.drive.pulse) {
            self.apply(step);
        }
        if self.drive.pulse == FILTER_SAMPLE {
            self.drive.filter_hits = self.order.len();
        }
        self.drive.pulse += 1;
        true
    }

    fn tick(&mut self) {
        let moved = tick(&mut self.prices, self.ticks);
        self.ticks += 1;
        self.moved += u64::try_from(moved).unwrap_or(0);
    }

    fn apply(&mut self, step: Step) {
        match step {
            Step::Scroll(rows) => {
                self.drive.scroll_rows += rows;
                let y = scroll_px(self.drive.scroll_rows);
                self.scroll
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.0), px(-y)));
            }
            Step::Sort(kind) => {
                self.sort = match self.sort {
                    Some((current, true)) if current == kind => Some((kind, false)),
                    Some((current, false)) if current == kind => None,
                    _ => Some((kind, true)),
                };
                self.reorder();
            }
            Step::Filter(text) => {
                self.filter = text.into();
                self.reorder();
            }
            Step::Select(row) => self.selected = Some(row),
            Step::Edit => {
                if let Some(row) = self.selected
                    && let Some(price) = self.price(row)
                {
                    self.draft = format!("{price:.3}");
                    self.editing = Some(row);
                }
            }
            Step::Commit(value) => {
                self.draft = format!("{value:.3}");
                self.commit();
            }
            Step::Undo => {
                if let Some((row, before)) = self.undo.pop() {
                    self.set(row, before);
                }
            }
            Step::Theme => self.dark = !self.dark,
        }
    }

    fn commit(&mut self) {
        let (Some(row), Some(value)) = (self.editing, parse_price(&self.draft)) else {
            return;
        };
        let Some(before) = self.price(row) else {
            return;
        };
        self.set(row, value);
        self.undo.push((row, before));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.editing = None;
        self.drive.edits += 1;
    }

    fn reorder(&mut self) {
        let needle = self.filter.as_str();
        let mut order: Vec<u32> = self
            .book
            .iter()
            .enumerate()
            .filter(|(_, row)| needle.is_empty() || row.matches(needle))
            .map(|(i, _)| u32::try_from(i).unwrap_or(u32::MAX))
            .collect();
        if let Some((kind, ascending)) = self.sort {
            order.sort_by(|a, b| self.compare(*a, *b, kind));
            if !ascending {
                order.reverse();
            }
        }
        self.order = order;
    }

    fn compare(&self, left: u32, right: u32, kind: FieldKind) -> Ordering {
        if kind == FieldKind::LastDone {
            return match (self.price(left), self.price(right)) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                _ => Ordering::Equal,
            };
        }
        match (self.row(left), self.row(right)) {
            (Some(x), Some(y)) => compare(x, y, kind),
            _ => Ordering::Equal,
        }
    }

    #[must_use]
    pub fn row(&self, index: u32) -> Option<&Row> {
        self.book.get(usize::try_from(index).ok()?)
    }

    #[must_use]
    pub fn price(&self, index: u32) -> Option<f64> {
        self.prices.get(usize::try_from(index).ok()?).copied()
    }

    fn set(&mut self, row: u32, value: f64) {
        if let Some(slot) = usize::try_from(row)
            .ok()
            .and_then(|i| self.prices.get_mut(i))
        {
            *slot = value;
        }
    }

    #[must_use]
    pub fn observed(&self) -> Tally {
        Tally {
            pulses: u32::try_from(self.drive.marks.len()).unwrap_or(u32::MAX),
            ticks: self.ticks,
            moved: self.moved,
            edits: self.drive.edits,
            undo_depth: self.undo.len(),
            visible: self.order.len(),
            filter_hits: self.drive.filter_hits,
            scroll_rows: self.drive.scroll_rows,
            stride: TICK_STRIDE,
        }
    }

    #[must_use]
    pub fn record(&self) -> Record {
        let mut record = Record::new("gpui", GPUI_VERSION, backend(), VIEWPORT);
        record.finish(SCENARIO, &self.drive.marks);
        record.checks = Tally::expect(&self.book).checks(&self.observed());
        record.notes.push(
            "Inline edit is a modeled cell (draft text in a bordered div); gpui 0.2.2 ships no text input widget."
                .into(),
        );
        record
    }
}

const fn backend() -> &'static str {
    if cfg!(target_os = "macos") {
        "metal"
    } else if cfg!(target_os = "windows") {
        "directx"
    } else {
        "blade-vulkan"
    }
}

fn summary(record: &Record) -> String {
    let s = record.summary;
    format!(
        "{} pulses, p50 {} us, p95 {} us, p99 {} us, max {} us, >60Hz {}, checks {}",
        record.pulses,
        s.p50_ns / 1000,
        s.p95_ns / 1000,
        s.p99_ns / 1000,
        s.max_ns / 1000,
        s.over_60hz,
        if record.all_pass() { "pass" } else { "FAIL" }
    )
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
