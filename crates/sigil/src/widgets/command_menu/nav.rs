use std::rc::Rc;

use crate::reactive::{Owner, Signal, signal};

use super::super::listbox::Matcher;
use super::super::popup::hide;
use super::super::shared::Activate;

pub(super) type SubmitObserver = Rc<dyn Fn(&str)>;

pub(super) enum PaletteEntry {
    Leaf {
        label: String,
        detail: Option<String>,
        action: Option<Activate>,
        disabled: bool,
    },
    Submenu {
        label: String,
        disabled: bool,
        entries: Rc<Vec<Self>>,
    },
}

impl PaletteEntry {
    pub(super) const fn disabled(&self) -> bool {
        match self {
            Self::Leaf { disabled, .. } | Self::Submenu { disabled, .. } => *disabled,
        }
    }

    pub(super) fn detail(&self) -> Option<&str> {
        match self {
            Self::Leaf { detail, .. } => detail.as_deref(),
            Self::Submenu { .. } => None,
        }
    }

    fn search_text(&self) -> String {
        self.detail().map_or_else(
            || self.label().to_owned(),
            |detail| format!("{} {detail}", self.label()),
        )
    }

    pub(super) fn label(&self) -> &str {
        match self {
            Self::Leaf { label, .. } | Self::Submenu { label, .. } => label,
        }
    }
}

#[derive(Clone)]
pub(super) struct MenuLevel {
    pub(super) entries: Rc<Vec<PaletteEntry>>,
    pub(super) shown: Signal<Vec<usize>>,
    pub(super) cursor: Signal<usize>,
    owner: Option<Owner>,
}

impl MenuLevel {
    pub(super) fn new(entries: Rc<Vec<PaletteEntry>>) -> Self {
        let shown: Vec<usize> = (0..entries.len()).collect();
        let cursor = signal(first_enabled(&entries, &shown));
        Self {
            entries,
            shown: signal(shown),
            cursor,
            owner: None,
        }
    }

    fn dispose(&self) {
        if let Some(owner) = self.owner {
            owner.dispose();
        }
    }

    pub(super) fn entry_at(&self, row: usize) -> Option<&PaletteEntry> {
        let index = *self.shown.peek().get(row)?;
        self.entries.get(index)
    }
}

fn first_enabled(entries: &[PaletteEntry], shown: &[usize]) -> usize {
    shown
        .iter()
        .position(|index| entries.get(*index).is_some_and(|e| !e.disabled()))
        .unwrap_or(0)
}

fn ranked(entries: &[PaletteEntry], query: &str) -> Vec<usize> {
    let texts: Vec<String> = entries.iter().map(PaletteEntry::search_text).collect();
    Matcher::Fuzzy.rank(query, texts.iter().map(String::as_str))
}

#[derive(Clone, Copy)]
pub(super) struct CommandPalette {
    pub(super) stack: Signal<Vec<MenuLevel>>,
    pub(super) query: Signal<String>,
    open: Signal<bool>,
    owner: Option<Owner>,
}

impl CommandPalette {
    pub(super) fn new(entries: Rc<Vec<PaletteEntry>>, open: Signal<bool>) -> Self {
        Self {
            stack: signal(vec![MenuLevel::new(entries)]),
            query: signal(String::new()),
            open,
            owner: Owner::current(),
        }
    }

    fn level(self, entries: &Rc<Vec<PaletteEntry>>) -> MenuLevel {
        let Some(parent) = self.owner.filter(|owner| owner.is_alive()) else {
            return MenuLevel::new(Rc::clone(entries));
        };
        let owner = Owner::new("command-menu-level", Some(parent));
        MenuLevel {
            owner: Some(owner),
            ..owner.with(|| MenuLevel::new(Rc::clone(entries)))
        }
    }

    fn drop_levels_from(self, depth: usize) {
        let mut dropped = Vec::new();
        self.stack
            .update(|levels| dropped = levels.split_off(depth.min(levels.len())));
        self.clear_query();
        dropped.iter().for_each(MenuLevel::dispose);
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
        self.drop_levels_from(1);
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
            PaletteEntry::Submenu { entries, .. } => {
                let child = self.level(entries);
                self.stack.update(|current| current.push(child));
                self.clear_query();
            }
            PaletteEntry::Leaf { action, label, .. } => {
                if let Some(cb) = action {
                    cb();
                }
                if let Some(cb) = on_submit {
                    cb(label);
                }
                self.reset();
                hide(self.open);
            }
        }
    }

    pub(super) fn ascend_or_cancel(self, on_cancel: Option<&Activate>) {
        let depth = self.stack.peek().len();
        if depth > 1 {
            self.drop_levels_from(depth - 1);
        } else {
            if let Some(cb) = on_cancel {
                cb();
            }
            hide(self.open);
        }
    }
}
