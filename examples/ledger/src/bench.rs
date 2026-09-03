//! Windowed replay of the shared Ledger script (`bench_spec::ledger`).
//!
//! One interval at `PULSE` ticks the price stream, applies the step for
//! that pulse, and stamps the wall clock. The record is written and the
//! window closes after the last pulse.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use bench_spec::Record;
use bench_spec::ledger::{
    FILTER_SAMPLE, PULSE, PULSES, SCENARIO, Step, Tally, VIEWPORT, WINDOW, scroll_px,
};
use data_table::ledger::TICK_STRIDE;
use data_table::{FIELDS, FieldKind, TOTAL_ITEMS};
use sigil::prelude::*;

use crate::state::App;
use crate::view::tree;

pub fn run(out: PathBuf) -> AppResult<()> {
    sigil::launch(
        ("Sigil - ledger bench", WINDOW.0, WINDOW.1),
        move |cx: &mut Cx| {
            let app = App::new(TOTAL_ITEMS);
            let root = cx.add(tree(&app));
            let driver = Rc::new(RefCell::new(Driver::new(app, out.clone())));
            cx.interval(PULSE, move || {
                if driver.borrow_mut().pulse() {
                    request_window_command(WindowCommand::Close);
                }
            });
            root
        },
    )
}

pub struct Driver {
    app: App,
    out: PathBuf,
    pulse: u32,
    marks: Vec<Instant>,
    edits: u32,
    filter_hits: usize,
    scroll_rows: u32,
}

impl Driver {
    #[must_use]
    pub fn new(app: App, out: PathBuf) -> Self {
        Self {
            app,
            out,
            pulse: 0,
            marks: Vec::with_capacity(PULSES as usize),
            edits: 0,
            filter_hits: 0,
            scroll_rows: 0,
        }
    }

    /// Runs one pulse. Returns true once the script is complete and the
    /// record is written.
    pub fn pulse(&mut self) -> bool {
        if self.pulse >= PULSES {
            return true;
        }
        self.marks.push(Instant::now());
        self.app.live.tick();
        if let Some(step) = bench_spec::ledger::step(self.pulse) {
            self.apply(step);
        }
        if self.pulse == FILTER_SAMPLE {
            self.filter_hits = self.app.table.visible().peek().len();
        }
        self.pulse += 1;
        if self.pulse == PULSES {
            let record = self.record();
            if let Err(err) = record.write(&self.out) {
                eprintln!("ledger bench: write {}: {err}", self.out.display());
            }
            eprintln!("ledger bench: {}", summary(&record));
            return true;
        }
        false
    }

    fn apply(&mut self, step: Step) {
        let app = &self.app;
        match step {
            Step::Scroll(rows) => {
                self.scroll_rows += rows;
                app.scroll.set(scroll_px(self.scroll_rows));
            }
            Step::Sort(kind) => app.table.toggle_sort(key(kind)),
            Step::Filter(text) => app.table.quick_filter().set(text.into()),
            Step::Select(row) => app.table.select_only(row),
            Step::Edit => app.edit_selected(),
            Step::Commit(value) => {
                let text = format!("{value:.3}");
                app.live.draft.set(text.clone());
                app.live.commit(&text);
                if app.live.editing.peek().is_none() {
                    self.edits += 1;
                }
            }
            Step::Undo => app.live.undo(),
            Step::Theme => app.toggle_theme(),
        }
    }

    #[must_use]
    pub fn observed(&self) -> Tally {
        let app = &self.app;
        Tally {
            pulses: u32::try_from(self.marks.len()).unwrap_or(u32::MAX),
            ticks: app.live.ticks.peek(),
            moved: app.live.moved.peek(),
            edits: self.edits,
            undo_depth: app.live.undo.peek().len(),
            visible: app.table.visible().peek().len(),
            filter_hits: self.filter_hits,
            scroll_rows: self.scroll_rows,
            stride: TICK_STRIDE,
        }
    }

    #[must_use]
    pub fn record(&self) -> Record {
        let backend = std::env::var("SIGIL_BACKEND").unwrap_or_else(|_| "wgpu".into());
        let mut record = Record::new("sigil", env!("CARGO_PKG_VERSION"), &backend, VIEWPORT);
        record.finish(SCENARIO, &self.marks);
        let expect = Tally::expect(&self.app.table.rows().peek());
        record.checks = expect.checks(&self.observed());
        record
    }
}

fn key(kind: FieldKind) -> &'static str {
    FIELDS
        .iter()
        .find(|field| field.kind == kind)
        .map_or("id", |field| field.key)
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
