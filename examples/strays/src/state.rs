//! One [`TableState`] over the live scan.
//!
//! Refresh replaces the rows. Selection survives it because [`Stray`]
//! carries a stable id, so the row a user picked stays picked even as
//! processes above it exit and the indices shift.

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

/// The whole app hangs off one table plus the policy it classifies against.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Rows, columns, sort, filter, and selection.
    pub(crate) table: TableState<Stray>,
    /// Directory root that counts as a source tree.
    pub(crate) root: PathBuf,
    /// Unattended run time a dev-tree server may reach before it is a stray.
    pub(crate) limit: Duration,
    /// Reads the machine. Behind a lock because the read happens on a worker
    /// thread — it forks `ps` and walks the process table, which is far too
    /// much to do between two frames.
    reader: Arc<Mutex<Scanner>>,
    /// The scan in flight, if any. One at a time.
    job: Rc<RefCell<Option<ScanJob>>>,
}

impl AppState {
    /// Build over a reader that has already scanned.
    ///
    /// The first scan reads the whole socket and process table and costs tens
    /// of milliseconds. Running it from the tree builder would mount the app
    /// and then freeze it; the caller does it before the terminal comes up.
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

    /// Build over rows the caller supplies.
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

    /// Start a scan on a worker thread, unless one is already running.
    pub(crate) fn refresh(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        let reader = Arc::clone(&self.reader);
        let job = work(move || {
            reader
                .lock()
                // A prior scan panic can poison the lock. Report no rows
                // instead of taking down the app.
                .map_or_else(|_| Vec::new(), |mut scanner| scanner.read())
        });
        *self.job.borrow_mut() = Some(job);
    }

    /// Take the rows from a finished scan. Cheap enough to call every frame.
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

    /// Count of rows the policy calls a stray.
    pub(crate) fn stray_count(&self) -> usize {
        self.table
            .rows()
            .peek()
            .iter()
            .filter(|row| row.verdict(&self.root, self.limit) == Verdict::Stray)
            .count()
    }

    /// Signal the selected row's process group, then re-scan.
    ///
    /// Only a [`Verdict::Stray`] row is killable. Anything else is either
    /// outside the dev roots or has somebody watching it.
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

/// Tone the state badge paints in.
const fn tone(verdict: Verdict) -> Tone {
    match verdict {
        Verdict::System => Tone::Neutral,
        Verdict::Held => Tone::Success,
        Verdict::Fresh => Tone::Info,
        Verdict::Stray => Tone::Warn,
    }
}

/// `1h05m`, or `47m` under the hour.
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
