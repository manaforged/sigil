use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sigil::prelude::Status;
use sigil::prelude::*;
use sigil::widgets::{Identifiable, TableColumn, TableState, badge, label};
use sigil::work;

use crate::scan::{self, Scanner, Stray, Verdict};

type ScanJob = Signal<Status<Vec<Stray>, String>>;

impl Identifiable for Stray {
    type Id = u32;

    fn id(&self) -> u32 {
        self.pid
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) table: TableState<Stray>,
    pub(crate) root: PathBuf,
    pub(crate) limit: Duration,
    reader: Arc<Mutex<Scanner>>,
    job: Rc<RefCell<Option<ScanJob>>>,
}

impl AppState {
    pub(crate) fn adopt(
        reader: Arc<Mutex<Scanner>>,
        rows: Vec<Stray>,
        root: PathBuf,
        limit: Duration,
    ) -> Self {
        let mut state = Self::from_rows(rows, root, limit);
        state.reader = reader;
        state
    }

    pub(crate) fn from_rows(rows: Vec<Stray>, root: PathBuf, limit: Duration) -> Self {
        let table = TableState::identified(rows, columns(root.clone(), limit));
        Self {
            table,
            root,
            limit,
            reader: Arc::new(Mutex::new(Scanner::new())),
            job: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn refresh(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        let reader = Arc::clone(&self.reader);
        let job = work(move || {
            reader
                .lock()
                .map_or_else(|_| Vec::new(), |mut scanner| scanner.read())
        });
        *self.job.borrow_mut() = Some(job);
    }

    pub(crate) fn land(&self) {
        let Some(job) = *self.job.borrow() else {
            return;
        };
        let Some(Status::Ok(rows)) = job.try_peek() else {
            return;
        };
        *self.job.borrow_mut() = None;
        self.table.replace_rows(rows);
    }

    pub(crate) fn stray_count(&self) -> usize {
        self.table
            .rows()
            .peek()
            .iter()
            .filter(|row| row.verdict(&self.root, self.limit) == Verdict::Stray)
            .count()
    }

    pub(crate) fn kill_selected(&self) -> Option<u32> {
        let row = self.table.selected_row()?;
        if row.verdict(&self.root, self.limit) != Verdict::Stray {
            return None;
        }
        scan::kill(row.pgid);
        self.refresh();
        Some(row.pid)
    }
}

const fn tone(verdict: Verdict) -> Tone {
    match verdict {
        Verdict::System => Tone::Neutral,
        Verdict::Held => Tone::Success,
        Verdict::Fresh => Tone::Info,
        Verdict::Stray => Tone::Warn,
    }
}

fn span(age: Duration) -> String {
    let secs = age.as_secs();
    match (secs / 3600, (secs % 3600) / 60) {
        (0, minutes) => format!("{minutes}m"),
        (hours, minutes) => format!("{hours}h{minutes:02}m"),
    }
}

fn columns(root: PathBuf, limit: Duration) -> Vec<TableColumn<Stray>> {
    let badge_root = root.clone();
    vec![
        TableColumn::new("pid", "PID")
            .width(ColWidth::Fixed(56.0))
            .sort_by(|row: &Stray| row.pid)
            .cell(|row: &Stray| label(row.pid.to_string())),
        TableColumn::new("port", "PORT")
            .width(ColWidth::Fixed(72.0))
            .sort_by(|row: &Stray| row.ports.first().copied().unwrap_or_default())
            .filterable(|row: &Stray, needle| row.port_list().contains(needle))
            .cell(|row: &Stray| label(row.port_list())),
        TableColumn::new("age", "AGE")
            .width(ColWidth::Fixed(64.0))
            .sort_by(|row: &Stray| row.age)
            .cell(|row: &Stray| label(span(row.age))),
        TableColumn::new("mem", "MEM")
            .width(ColWidth::Fixed(64.0))
            .sort_by(|row: &Stray| row.rss)
            .cell(|row: &Stray| label(format!("{}M", row.rss / 1_048_576))),
        TableColumn::new("state", "STATE")
            .width(ColWidth::Fixed(72.0))
            .sort_by(move |row: &Stray| row.verdict(&badge_root, limit))
            .cell(move |row: &Stray| {
                let verdict = row.verdict(&root, limit);
                badge(verdict.title(), tone(verdict))
            }),
        TableColumn::new("where", "WHERE")
            .width(ColWidth::Fixed(176.0))
            .sort_by(Stray::place)
            .filterable(|row: &Stray, needle| row.place().contains(needle))
            .cell(|row: &Stray| label(row.place())),
        TableColumn::new("cmd", "COMMAND")
            .width(ColWidth::Fraction(1.0))
            .filterable(|row: &Stray, needle| row.cmd.contains(needle))
            .cell(|row: &Stray| label(row.cmd.clone())),
    ]
}
