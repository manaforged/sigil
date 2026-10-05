use sigil::prelude::*;
use sigil::widgets::{ChangeKind, DiagSpan, HlSpan};

use super::facts::{CHANGES, DIAGNOSTICS};
use super::project::{FILE_COUNT, FileId, Folder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Activity {
    Explorer,
    Search,
    SourceControl,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BottomTab {
    Problems,
    Output,
    Terminal,
}

impl BottomTab {
    pub(crate) const ALL: [Self; 3] = [Self::Problems, Self::Output, Self::Terminal];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Problems => "Problems",
            Self::Output => "Output",
            Self::Terminal => "Terminal",
        }
    }

    pub(crate) fn at(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Problems)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Buffer {
    pub(crate) lines: Signal<Vec<String>>,
    pub(crate) caret: Signal<(usize, usize)>,
    pub(crate) highlights: Signal<Vec<Vec<HlSpan>>>,
    pub(crate) gutter: Signal<Vec<Option<ChangeKind>>>,
    pub(crate) diagnostics: Signal<Vec<DiagSpan>>,
    pub(crate) text: Signal<String>,
}

impl Buffer {
    fn load(file: FileId) -> Self {
        let spec = file.spec();
        let source = spec.source.trim_end();
        let lines: Vec<String> = source.lines().map(String::from).collect();
        let total = lines.len();
        let lines = signal(lines);
        let language = spec.kind.language();
        let highlights = signal(Vec::new());
        effect(move || {
            let spans = lines.with(|rows| hl_spans(language, &rows.join("\n")));
            highlights.update(|current| *current = spans);
        });
        Self {
            lines,
            caret: signal((0, 0)),
            highlights,
            gutter: signal(gutter_marks(file, total)),
            diagnostics: signal(diag_spans(file)),
            text: derived(move || lines.with(|rows| rows.join("\n"))),
        }
    }
}

fn gutter_marks(file: FileId, total: usize) -> Vec<Option<ChangeKind>> {
    (0..total)
        .map(|row| {
            CHANGES
                .iter()
                .find(|change| change.file == file && (change.first..=change.last).contains(&row))
                .map(|change| change.kind)
        })
        .collect()
}

fn diag_spans(file: FileId) -> Vec<DiagSpan> {
    DIAGNOSTICS
        .iter()
        .filter(|diag| diag.file == file)
        .map(|diag| DiagSpan {
            line: diag.line,
            start: diag.start,
            end: diag.end,
            severity: diag.severity,
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct Workspace {
    pub(crate) buffers: Signal<[Buffer; FILE_COUNT]>,
    pub(crate) open: Signal<Vec<FileId>>,
    pub(crate) active_tab: Signal<usize>,
    pub(crate) expanded: Signal<Vec<Folder>>,
    pub(crate) dirty: Signal<Vec<FileId>>,
    pub(crate) activity: Signal<Activity>,
    pub(crate) bottom: Signal<usize>,
    pub(crate) staged: Signal<Vec<FileId>>,
    pub(crate) outline_open: Signal<bool>,
    pub(crate) palette_open: Signal<bool>,
    pub(crate) quick_open: Signal<bool>,
    pub(crate) terminal: Signal<Vec<String>>,
    pub(crate) toasts: ToastQueue,
}

pub(crate) const START_OPEN: [FileId; 3] = [FileId::Integrator, FileId::Main, FileId::Readme];
pub(crate) const START_CARET: (usize, usize) = (41, 16);

impl Workspace {
    pub(crate) fn new() -> Self {
        let buffers = FileId::ALL.map(Buffer::load);
        let ws = Self {
            buffers: signal(buffers),
            open: signal(START_OPEN.to_vec()),
            active_tab: signal(0),
            expanded: signal(vec![Folder::Src, Folder::Physics, Folder::Tests]),
            dirty: signal(
                FileId::ALL
                    .into_iter()
                    .filter(|file| file.spec().dirty)
                    .collect(),
            ),
            activity: signal(Activity::Explorer),
            bottom: signal(0),
            staged: signal(Vec::new()),
            outline_open: signal(true),
            palette_open: signal(false),
            quick_open: signal(false),
            terminal: signal(
                super::facts::TERMINAL
                    .iter()
                    .map(|line| String::from(*line))
                    .collect(),
            ),
            toasts: ToastQueue::new(),
        };
        ws.buffer(FileId::Integrator).caret.set(START_CARET);
        ws
    }

    pub(crate) fn buffer(&self, file: FileId) -> Buffer {
        self.buffers
            .with(|all| all.get(file.index()).copied())
            .unwrap_or_else(|| Buffer::load(file))
    }

    pub(crate) fn active(&self) -> Option<FileId> {
        let tab = self.active_tab.get();
        self.open.with(|open| open.get(tab).copied())
    }

    pub(crate) fn open_file(&self, file: FileId) {
        let existing = self.open.with(|open| open.iter().position(|f| *f == file));
        let index = existing.unwrap_or_else(|| {
            self.open.update(|open| open.push(file));
            self.open.with(Vec::len).saturating_sub(1)
        });
        self.active_tab.set(index);
    }

    pub(crate) fn goto(&self, file: FileId, line: usize, col: usize) {
        self.open_file(file);
        self.buffer(file).caret.set((line, col));
    }

    pub(crate) fn close_tab(&self, index: usize) {
        self.open.update(|open| {
            if index < open.len() {
                open.remove(index);
            }
        });
        let len = self.open.with(Vec::len);
        let active = self.active_tab.get();
        if active >= len || active > index {
            self.active_tab.set(active.saturating_sub(1));
        }
    }

    pub(crate) fn toggle_folder(&self, folder: Folder) {
        self.expanded.update(|open| {
            if let Some(pos) = open.iter().position(|f| *f == folder) {
                open.remove(pos);
            } else {
                open.push(folder);
            }
        });
    }

    pub(crate) fn is_dirty(&self, file: FileId) -> bool {
        self.dirty.with(|dirty| dirty.contains(&file))
    }

    pub(crate) fn save_active(&self) {
        if let Some(file) = self.active() {
            self.dirty.update(|dirty| dirty.retain(|f| *f != file));
            self.toasts.success(format!("Saved {}", file.path()));
        }
    }

    pub(crate) fn toggle_stage(&self, file: FileId) {
        self.staged.update(|staged| {
            if let Some(pos) = staged.iter().position(|f| *f == file) {
                staged.remove(pos);
            } else {
                staged.push(file);
            }
        });
    }

    pub(crate) fn show_bottom(&self, tab: BottomTab) {
        let index = BottomTab::ALL.iter().position(|t| *t == tab).unwrap_or(0);
        self.bottom.set(index);
    }
}
