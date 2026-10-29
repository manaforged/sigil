use std::cell::Cell;
use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;
use std::rc::Rc;

use crate::reactive::{Signal, effect, untrack};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mark {
    Empty,
    Partial,
    Full,
}

pub(crate) trait KeySet<K>: Extend<K> {
    fn holds(&self, key: &K) -> bool;
    fn put(&mut self, key: K);
    fn take(&mut self, key: &K) -> bool;
    fn wipe(&mut self);
    fn is_blank(&self) -> bool;
}

impl<K: Eq + Hash> KeySet<K> for HashSet<K> {
    fn holds(&self, key: &K) -> bool {
        self.contains(key)
    }
    fn put(&mut self, key: K) {
        self.insert(key);
    }
    fn take(&mut self, key: &K) -> bool {
        self.remove(key)
    }
    fn wipe(&mut self) {
        self.clear();
    }
    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

impl<K: Ord> KeySet<K> for BTreeSet<K> {
    fn holds(&self, key: &K) -> bool {
        self.contains(key)
    }
    fn put(&mut self, key: K) {
        self.insert(key);
    }
    fn take(&mut self, key: &K) -> bool {
        self.remove(key)
    }
    fn wipe(&mut self) {
        self.clear();
    }
    fn is_blank(&self) -> bool {
        self.is_empty()
    }
}

pub(crate) fn only<K: Copy>(set: &mut impl KeySet<K>, anchor: &mut Option<K>, key: K) {
    set.wipe();
    set.put(key);
    *anchor = Some(key);
}

pub(crate) fn toggle<K>(set: &mut impl KeySet<K>, key: K) {
    if !set.take(&key) {
        set.put(key);
    }
}

pub(crate) fn span<K>(set: &mut impl KeySet<K>, keys: impl IntoIterator<Item = K>) {
    set.wipe();
    set.extend(keys);
}

pub(crate) fn toggle_all<K: Copy>(set: &mut impl KeySet<K>, shown: &[K]) {
    if !shown.is_empty() && shown.iter().all(|key| set.holds(key)) {
        for key in shown {
            set.take(key);
        }
    } else {
        set.extend(shown.iter().copied());
    }
}

pub(crate) fn mark<K>(set: &impl KeySet<K>, shown: &[K]) -> Mark {
    if set.is_blank() {
        return Mark::Empty;
    }
    let mut marks = shown.iter().map(|key| set.holds(key));
    match marks.next() {
        Some(first) if marks.any(|selected| selected != first) => Mark::Partial,
        Some(true) => Mark::Full,
        None | Some(false) => Mark::Empty,
    }
}

pub(crate) fn follow_cursor<K: Clone + Eq + 'static>(
    cursor: Signal<usize>,
    selected: Signal<Option<K>>,
    key_at: impl Fn(usize) -> Option<K> + 'static,
    index_of: impl Fn(&K) -> Option<usize> + 'static,
) {
    let seen: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    effect(move || {
        let index = cursor.get();
        if seen.replace(Some(index)).is_none() {
            return;
        }
        let id = untrack(|| key_at(index));
        if id.is_some() && selected.peek() != id {
            selected.set(id);
        }
    });
    effect(move || {
        let Some(id) = selected.get() else {
            return;
        };
        if let Some(index) = index_of(&id)
            && cursor.peek() != index
        {
            cursor.set(index);
        }
    });
}
