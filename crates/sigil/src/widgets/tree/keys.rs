use crate::events::{Key, KeyEvent, LogicalKey, NodeEventsExt};
use crate::kernel::NodeBuilder;

use super::super::listbox::{Move, Rows, TREE, follow_cursor};
use super::super::patterns::Disclosure;
use super::{TreeKey, TreeRow, TreeState};

pub(super) fn sync_cursor<K: TreeKey>(state: &TreeState<K>) {
    let rows = state.rows;
    follow_cursor(
        state.cursor,
        state.selected,
        move |index| rows.with(|rows| rows.get(index).map(|row| row.id.clone())),
        move |id| rows.with(|rows| rows.iter().position(|row| row.id == *id)),
    );
}

struct Flat<'a, K: TreeKey>(&'a [TreeRow<K>]);

impl<K: TreeKey> Rows for Flat<'_, K> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn parent(&self, index: usize) -> Option<usize> {
        self.0.get(index).and_then(|row| row.parent)
    }

    fn child(&self, index: usize) -> Option<usize> {
        self.0
            .get(index + 1)
            .is_some_and(|next| next.parent == Some(index))
            .then_some(index + 1)
    }
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
    Level(Move),
    Activate,
    Toggle,
}

pub(super) fn attach<K: TreeKey>(node: NodeBuilder, state: &TreeState<K>) -> NodeBuilder {
    let bind = |command: Command| {
        let state = state.clone();
        move |event: &mut KeyEvent| run(&state, command, event)
    };
    let mut node = node;
    for (key, step) in TREE.bindings() {
        if matches!(step, Move::Parent | Move::Child) {
            node = node.key(key, bind(Command::Level(step)));
        }
    }
    node.key(Key::Enter, bind(Command::Activate))
        .key(LogicalKey::character(" "), bind(Command::Toggle))
}

fn run<K: TreeKey>(state: &TreeState<K>, command: Command, event: &mut KeyEvent) {
    let index = state.cursor.peek();
    let Some(row) = state.rows.with(|rows| rows.get(index).cloned()) else {
        return;
    };
    event.stop_propagation();
    match command {
        Command::Level(Move::Parent) if row.disclosure == Disclosure::Expanded => {
            toggle(state, &row);
        }
        Command::Level(Move::Child) if row.disclosure == Disclosure::Collapsed => {
            toggle(state, &row);
        }
        Command::Level(step) => {
            let target = state
                .rows
                .with(|rows| TREE.target(&Flat(rows), index, step));
            if let Some(target) = target {
                state.cursor.set(target);
            }
        }
        Command::Activate => activate(state, &row),
        Command::Toggle if row.has_children() => toggle(state, &row),
        Command::Toggle => select(state, index, &row),
    }
}
