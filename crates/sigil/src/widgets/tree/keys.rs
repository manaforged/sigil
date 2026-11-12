use crate::events::{Key, KeyEvent, NamedKey};
use crate::kernel::NodeBuilder;
use crate::widgets::Widget;

use super::super::listbox::{Move, RowSource, TREE, follow_cursor};
use super::super::patterns::Disclosure;
use core::hash::Hash;

use super::{TreeRow, TreeState};

pub(super) fn sync_cursor<K: Clone + Eq + Hash + 'static>(state: &TreeState<K>) {
    let rows = state.rows;
    follow_cursor(
        state.cursor,
        state.selected,
        move |index| rows.with(|rows| rows.get(index).map(|row| row.id.clone())),
        move |id| rows.with(|rows| rows.iter().position(|row| row.id == *id)),
    );
}

struct Flat<'a, K: Clone + Eq + Hash + 'static>(&'a [TreeRow<K>]);

impl<K: Clone + Eq + Hash + 'static> RowSource for Flat<'_, K> {
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

pub(super) fn toggle<K: Clone + Eq + Hash + 'static>(state: &TreeState<K>, row: &TreeRow<K>) {
    let id = row.id.clone();
    match row.disclosure {
        Disclosure::Expanded => state.expanded.update(|open| open.retain(|key| *key != id)),
        Disclosure::Collapsed => state.expanded.update(|open| open.push(id)),
        Disclosure::Leaf | Disclosure::None => {}
    }
}

pub(super) fn activate<K: Clone + Eq + Hash + 'static>(state: &TreeState<K>, row: &TreeRow<K>) {
    match &state.activate {
        Some(activate) => activate(&row.id),
        None => toggle(state, row),
    }
}

pub(super) fn select<K: Clone + Eq + Hash + 'static>(
    state: &TreeState<K>,
    index: usize,
    row: &TreeRow<K>,
) {
    if state.selected.peek().as_ref() != Some(&row.id) {
        state.selected.set(Some(row.id.clone()));
    }
    if state.cursor.peek() != index {
        state.cursor.set(index);
    }
}

#[derive(Clone, Copy)]
enum TreeCommand {
    Level(Move),
    Activate,
    Toggle,
}

pub(super) fn attach<K: Clone + Eq + Hash + 'static>(
    node: NodeBuilder,
    state: &TreeState<K>,
) -> NodeBuilder {
    let bind = |command: TreeCommand| {
        let state = state.clone();
        move |event: &mut KeyEvent| run(&state, command, event)
    };
    let mut node = node;
    for (key, step) in TREE.bindings() {
        if matches!(step, Move::Parent | Move::Child) {
            node = node.key(key, bind(TreeCommand::Level(step)));
        }
    }
    node.key(NamedKey::Enter, bind(TreeCommand::Activate))
        .key(Key::character(" "), bind(TreeCommand::Toggle))
}

fn run<K: Clone + Eq + Hash + 'static>(
    state: &TreeState<K>,
    command: TreeCommand,
    event: &mut KeyEvent,
) {
    let index = state.cursor.peek();
    let Some(row) = state.rows.with(|rows| rows.get(index).cloned()) else {
        return;
    };
    event.stop_propagation();
    match command {
        TreeCommand::Level(Move::Parent) if row.disclosure == Disclosure::Expanded => {
            toggle(state, &row);
        }
        TreeCommand::Level(Move::Child) if row.disclosure == Disclosure::Collapsed => {
            toggle(state, &row);
        }
        TreeCommand::Level(step) => {
            let target = state
                .rows
                .with(|rows| TREE.target(&Flat(rows), index, step));
            if let Some(target) = target {
                state.cursor.set(target);
            }
        }
        TreeCommand::Activate => activate(state, &row),
        TreeCommand::Toggle if row.has_children() => toggle(state, &row),
        TreeCommand::Toggle => select(state, index, &row),
    }
}
