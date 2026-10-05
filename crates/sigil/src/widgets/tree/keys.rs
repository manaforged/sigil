use std::cell::Cell;
use std::rc::Rc;

use crate::events::{Key, KeyEvent, LogicalKey, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::{effect, untrack};

use super::super::patterns::Disclosure;
use super::{TreeKey, TreeRow, TreeState};

pub(super) fn sync_cursor<K: TreeKey>(state: &TreeState<K>) {
    let (rows, selected, cursor) = (state.rows, state.selected, state.cursor);
    let seen: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    effect(move || {
        let index = cursor.get();
        let first = seen.replace(Some(index)).is_none();
        if first {
            return;
        }
        let id = untrack(|| rows.with(|rows| rows.get(index).map(|row| row.id.clone())));
        if id.is_some() && selected.peek() != id {
            selected.set(id);
        }
    });
    effect(move || {
        let Some(id) = selected.get() else {
            return;
        };
        let found = rows.with(|rows| rows.iter().position(|row| row.id == id));
        if let Some(index) = found
            && cursor.peek() != index
        {
            cursor.set(index);
        }
    });
}

pub(super) fn toggle<K: TreeKey>(state: &TreeState<K>, row: &TreeRow<K>) {
    let id = row.id.clone();
    match row.disclosure {
        Disclosure::Expanded => state.expanded.update(|open| open.retain(|key| *key != id)),
        Disclosure::Collapsed => state.expanded.update(|open| open.push(id)),
        Disclosure::Leaf | Disclosure::None => {}
    }
}

pub(super) fn activate<K: TreeKey>(state: &TreeState<K>, row: &TreeRow<K>) {
    match &state.activate {
        Some(activate) => activate(&row.id),
        None => toggle(state, row),
    }
}

pub(super) fn select<K: TreeKey>(state: &TreeState<K>, index: usize, row: &TreeRow<K>) {
    if state.selected.peek().as_ref() != Some(&row.id) {
        state.selected.set(Some(row.id.clone()));
    }
    if state.cursor.peek() != index {
        state.cursor.set(index);
    }
}

#[derive(Clone, Copy)]
enum Command {
    Collapse,
    Expand,
    Activate,
    Toggle,
}

pub(super) fn attach<K: TreeKey>(node: NodeBuilder, state: &TreeState<K>) -> NodeBuilder {
    let bind = |command: Command| {
        let state = state.clone();
        move |event: &mut KeyEvent| run(&state, command, event)
    };
    node.key(Key::ArrowLeft, bind(Command::Collapse))
        .key(Key::ArrowRight, bind(Command::Expand))
        .key(Key::Enter, bind(Command::Activate))
        .key(LogicalKey::character(" "), bind(Command::Toggle))
}

fn run<K: TreeKey>(state: &TreeState<K>, command: Command, event: &mut KeyEvent) {
    let index = state.cursor.peek();
    let Some(row) = state.rows.with(|rows| rows.get(index).cloned()) else {
        return;
    };
    event.stop_propagation();
    match command {
        Command::Collapse if row.disclosure == Disclosure::Expanded => toggle(state, &row),
        Command::Collapse => {
            if let Some(parent) = row.parent {
                state.cursor.set(parent);
            }
        }
        Command::Expand if row.disclosure == Disclosure::Collapsed => toggle(state, &row),
        Command::Expand => {
            let child = state.rows.with(|rows| {
                rows.get(index + 1)
                    .is_some_and(|next| next.parent == Some(index))
            });
            if child {
                state.cursor.set(index + 1);
            }
        }
        Command::Activate => activate(state, &row),
        Command::Toggle if row.has_children() => toggle(state, &row),
        Command::Toggle => select(state, index, &row),
    }
}
