use crate::reactive::{Signal, signal};
use core::hash::Hash;

use super::Matcher;

/// What a list cursor does at the end of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorEdge {
    /// The cursor stops at the first and the last row.
    Clamp,
    /// The cursor moves from the last row to the first, and back.
    Wrap,
}

/// The cursor, selection, expansion, and query of a list.
#[non_exhaustive]
pub struct ListState<K: Clone + Eq + Hash + 'static = String> {
    /// The index of the row under the cursor.
    pub cursor: Signal<usize>,
    /// The scroll offset in pixels.
    pub scroll: Signal<f32>,
    /// The keys of the expanded rows.
    pub expanded: Signal<Vec<K>>,
    /// The type-ahead or filter text.
    pub query: Signal<String>,
    /// The caret position in the query, in bytes.
    pub caret: Signal<usize>,
}

impl<K: Clone + Eq + Hash + 'static> core::fmt::Debug for ListState<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ListState")
            .field("cursor", &self.cursor)
            .field("scroll", &self.scroll)
            .field("expanded", &self.expanded)
            .field("query", &self.query)
            .field("caret", &self.caret)
            .finish()
    }
}

#[expect(
    clippy::expl_impl_clone_on_copy,
    reason = "a derive would require K: Copy, and Signal<K> is Copy for every K"
)]
impl<K: Clone + Eq + Hash + 'static> Clone for ListState<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: Clone + Eq + Hash + 'static> Copy for ListState<K> {}

impl<K: Clone + Eq + Hash + 'static> Default for ListState<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone + Eq + Hash + 'static> ListState<K> {
    /// Creates a list state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cursor: signal(0),
            scroll: signal(0.0),
            expanded: signal(Vec::new()),
            query: signal(String::new()),
            caret: signal(0),
        }
    }

    /// Returns the cursor index clamped to `len`.
    #[must_use]
    pub fn at(self, len: usize) -> usize {
        self.cursor.peek().min(len.saturating_sub(1))
    }

    /// Moves the cursor by `delta`.
    pub fn shift(self, delta: isize, len: usize, edge: CursorEdge) {
        let at = self.at(len);
        let next = match edge {
            CursorEdge::Clamp => at.saturating_add_signed(delta).min(len.saturating_sub(1)),
            CursorEdge::Wrap => wrap(at, delta, len),
        };
        self.cursor.set(next);
    }

    /// Moves the cursor to `index`.
    pub fn select(self, index: usize) {
        self.cursor.set(index);
    }

    /// Returns whether this list state is expanded.
    #[must_use]
    pub fn is_expanded(self, key: &K) -> bool {
        self.expanded.with(|held| held.contains(key))
    }

    /// Flips the expansion of `key`.
    pub fn toggle(self, key: &K) {
        let mut held = self.expanded.peek();
        match held.iter().position(|item| item == key) {
            Some(index) => drop(held.remove(index)),
            None => held.push(key.clone()),
        }
        self.expanded.set(held);
    }

    /// Expands `key` and collapses the others.
    pub fn toggle_only(self, key: &K) {
        let next = if self.is_expanded(key) {
            Vec::new()
        } else {
            vec![key.clone()]
        };
        self.expanded.set(next);
    }

    /// Collapses every key.
    pub fn collapse_all(self) {
        self.expanded.set(Vec::new());
    }

    /// Adds `text` to the type-ahead query.
    pub fn type_query(self, text: String) {
        self.caret.set(text.len());
        self.query.set(text);
    }

    /// Returns the indexes of labels that match the query.
    #[must_use]
    pub fn filter<'a>(self, labels: impl IntoIterator<Item = &'a str>) -> Vec<usize> {
        let query = self.query.peek();
        Matcher::Fuzzy.rank(&query, labels)
    }
}

fn wrap(at: usize, delta: isize, len: usize) -> usize {
    let len = isize::try_from(len.max(1)).unwrap_or(isize::MAX);
    let at = isize::try_from(at).unwrap_or(0);
    usize::try_from(at.saturating_add(delta).rem_euclid(len)).unwrap_or(0)
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
