use std::rc::Rc;

use crate::reactive::{Owner, OwnerId, Signal, owner_by_id, signal};

use super::super::fuzzy::fuzzy_score;
use super::super::shared::Activate;

pub(super) type SubmitObserver = Rc<dyn Fn(&str)>;

pub(super) enum Entry {
    Leaf {
        label: String,
        action: Option<Activate>,
        disabled: bool,
    },
    Submenu {
        label: String,
        disabled: bool,
        entries: Rc<Vec<Self>>,
    },
}

impl Entry {
    pub(super) const fn disabled(&self) -> bool {
        match self {
            Self::Leaf { disabled, .. } | Self::Submenu { disabled, .. } => *disabled,
        }
    }

    pub(super) fn label(&self) -> &str {
        match self {
            Self::Leaf { label, .. } | Self::Submenu { label, .. } => label,
        }
    }
}

#[derive(Clone)]
pub(super) struct MenuLevel {
    pub(super) entries: Rc<Vec<Entry>>,
    pub(super) shown: Signal<Vec<usize>>,
    pub(super) cursor: Signal<usize>,
}

impl MenuLevel {
    pub(super) fn new(entries: Rc<Vec<Entry>>) -> Self {
        let shown: Vec<usize> = (0..entries.len()).collect();
        let cursor = signal(first_enabled(&entries, &shown));
        Self {
            entries,
            shown: signal(shown),
            cursor,
        }
    }

    pub(super) fn entry_at(&self, row: usize) -> Option<&Entry> {
        let index = *self.shown.peek().get(row)?;
        self.entries.get(index)
    }
}

fn first_enabled(entries: &[Entry], shown: &[usize]) -> usize {
    shown
        .iter()
        .position(|index| entries.get(*index).is_some_and(|e| !e.disabled()))
        .unwrap_or(0)
}

fn ranked(entries: &[Entry], query: &str) -> Vec<usize> {
    let mut hits: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| fuzzy_score(query, entry.label()).map(|m| (m.score, index)))
        .collect();
    hits.sort_by_key(|(score, _)| *score);
    hits.into_iter().map(|(_, index)| index).collect()
}

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub(super) stack: Signal<Vec<MenuLevel>>,
    pub(super) query: Signal<String>,
    owner: Option<OwnerId>,
}

impl Palette {
    pub(super) fn new(entries: Rc<Vec<Entry>>) -> Self {
        Self {
            stack: signal(vec![MenuLevel::new(entries)]),
            query: signal(String::new()),
            owner: Owner::current(),
        }
    }

    fn level(self, entries: &Rc<Vec<Entry>>) -> MenuLevel {
        let mut made = None;
        if let Some(owner) = self.owner {
            let _ = owner_by_id(owner, || made = Some(MenuLevel::new(Rc::clone(entries))));
        }
        made.unwrap_or_else(|| MenuLevel::new(Rc::clone(entries)))
    }

    pub(super) fn top(self) -> Option<MenuLevel> {
        self.stack.peek().last().cloned()
    }

    pub(super) fn refilter(self) {
        let Some(top) = self.top() else {
            return;
        };
        let shown = ranked(&top.entries, &self.query.peek());
        top.cursor.set(first_enabled(&top.entries, &shown));
        top.shown.set(shown);
    }

    fn clear_query(self) {
        if !self.query.peek().is_empty() {
            self.query.set(String::new());
        }
        self.refilter();
    }

    pub(super) fn reset(self) {
        self.stack.update(|levels| levels.truncate(1));
        self.clear_query();
    }

    pub(super) fn activate(self, on_submit: Option<&SubmitObserver>) {
        let Some(top) = self.top() else {
            return;
        };
        let Some(entry) = top.entry_at(top.cursor.peek()) else {
            return;
        };
        if entry.disabled() {
            return;
        }
        match entry {
            Entry::Submenu { entries, .. } => {
                let child = self.level(entries);
                self.stack.update(|current| current.push(child));
                self.clear_query();
            }
            Entry::Leaf { action, label, .. } => {
                if let Some(cb) = action {
                    cb();
                }
                if let Some(cb) = on_submit {
                    cb(label);
                }
                self.reset();
            }
        }
    }

    pub(super) fn ascend_or_cancel(self, on_cancel: Option<&Activate>) {
        if self.stack.peek().len() > 1 {
            self.stack.update(|current| {
                current.pop();
            });
            self.clear_query();
        } else if let Some(cb) = on_cancel {
            cb();
        }
    }
}
