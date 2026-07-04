//! Command menu navigation runtime — the author-time [`LevelDef`] tree
//! converted into a reactive `Signal<Vec<MenuLevel>>` stack, plus the
//! Up/Down/Enter/Right/Esc/Left action functions [`super::render`]
//! wires to keyboard input.

use std::rc::Rc;

use reactive::{Signal, signal};

use super::super::shared::Activate;
use super::builder::{EntryDef, LevelDef};

/// Observer invoked with a leaf row's label on every activation.
pub(super) type SelectObserver = Rc<dyn Fn(&str)>;

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
    pub(super) cursor: Signal<usize>,
}

impl MenuLevel {
    pub(super) fn new(entries: Rc<Vec<Entry>>) -> Self {
        let cursor = signal(first_enabled(&entries));
        Self { entries, cursor }
    }
}

pub(super) fn into_runtime(def: LevelDef) -> Rc<Vec<Entry>> {
    Rc::new(
        def.entries
            .into_iter()
            .map(|entry| match entry {
                EntryDef::Leaf {
                    label,
                    action,
                    disabled,
                } => Entry::Leaf {
                    label,
                    action,
                    disabled,
                },
                EntryDef::Submenu {
                    label,
                    disabled,
                    def: nested,
                } => Entry::Submenu {
                    label,
                    disabled,
                    entries: into_runtime(nested),
                },
            })
            .collect(),
    )
}

pub(super) fn first_enabled(entries: &[Entry]) -> usize {
    entries.iter().position(|e| !e.disabled()).unwrap_or(0)
}

/// Next non-disabled index from `from`, walking forward or backward.
/// Clamps at the ends — stays at `from` when no further enabled row
/// exists in that direction.
fn next_enabled(entries: &[Entry], from: usize, forward: bool) -> usize {
    if entries.is_empty() {
        return from;
    }
    if forward {
        ((from + 1)..entries.len())
            .find(|&i| entries.get(i).is_some_and(|e| !e.disabled()))
            .unwrap_or(from)
    } else {
        (0..from)
            .rev()
            .find(|&i| entries.get(i).is_some_and(|e| !e.disabled()))
            .unwrap_or(from)
    }
}

pub(super) fn move_cursor(stack: Signal<Vec<MenuLevel>>, forward: bool) {
    let levels = stack.peek();
    let Some(top) = levels.last() else {
        return;
    };
    let next = next_enabled(&top.entries, top.cursor.peek(), forward);
    top.cursor.set(next);
}

fn reset_to_root(stack: Signal<Vec<MenuLevel>>) {
    stack.update(|levels| {
        levels.truncate(1);
        if let Some(root) = levels.first() {
            root.cursor.set(first_enabled(&root.entries));
        }
    });
}

pub(super) fn activate_or_descend(
    stack: Signal<Vec<MenuLevel>>,
    on_select: Option<&SelectObserver>,
) {
    let levels = stack.peek();
    let Some(top) = levels.last() else {
        return;
    };
    let idx = top.cursor.peek();
    let Some(entry) = top.entries.get(idx) else {
        return;
    };
    if entry.disabled() {
        return;
    }
    match entry {
        Entry::Submenu { entries, .. } => {
            let child = MenuLevel::new(Rc::clone(entries));
            stack.update(|current| current.push(child));
        }
        Entry::Leaf { action, label, .. } => {
            if let Some(cb) = action {
                cb();
            }
            if let Some(cb) = on_select {
                cb(label);
            }
            reset_to_root(stack);
        }
    }
}

pub(super) fn ascend_or_cancel(stack: Signal<Vec<MenuLevel>>, on_cancel: Option<&Activate>) {
    if stack.peek().len() > 1 {
        stack.update(|current| {
            current.pop();
        });
    } else if let Some(cb) = on_cancel {
        cb();
    }
}
