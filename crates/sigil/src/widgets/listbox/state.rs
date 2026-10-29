use crate::reactive::{Signal, signal};
use crate::widgets::tree::TreeKey;

use super::Matcher;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorEdge {
    Clamp,
    Wrap,
}

pub struct ListState<K: TreeKey = String> {
    pub cursor: Signal<usize>,
    pub scroll: Signal<f32>,
    pub expanded: Signal<Vec<K>>,
    pub query: Signal<String>,
    pub caret: Signal<usize>,
}

#[expect(
    clippy::expl_impl_clone_on_copy,
    reason = "a derive would require K: Copy, and Signal<K> is Copy for every K"
)]
impl<K: TreeKey> Clone for ListState<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: TreeKey> Copy for ListState<K> {}

impl<K: TreeKey> Default for ListState<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: TreeKey> ListState<K> {
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

    #[must_use]
    pub fn at(self, len: usize) -> usize {
        self.cursor.peek().min(len.saturating_sub(1))
    }

    pub fn shift(self, delta: isize, len: usize, edge: CursorEdge) {
        let at = self.at(len);
        let next = match edge {
            CursorEdge::Clamp => at.saturating_add_signed(delta).min(len.saturating_sub(1)),
            CursorEdge::Wrap => wrap(at, delta, len),
        };
        self.cursor.set(next);
    }

    pub fn select(self, index: usize) {
        self.cursor.set(index);
    }

    #[must_use]
    pub fn is_expanded(self, key: &K) -> bool {
        self.expanded.with(|held| held.contains(key))
    }

    pub fn toggle(self, key: &K) {
        let mut held = self.expanded.peek();
        match held.iter().position(|item| item == key) {
            Some(index) => drop(held.remove(index)),
            None => held.push(key.clone()),
        }
        self.expanded.set(held);
    }

    pub fn toggle_only(self, key: &K) {
        let next = if self.is_expanded(key) {
            Vec::new()
        } else {
            vec![key.clone()]
        };
        self.expanded.set(next);
    }

    pub fn collapse_all(self) {
        self.expanded.set(Vec::new());
    }

    pub fn type_query(self, text: String) {
        self.caret.set(text.len());
        self.query.set(text);
    }

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
