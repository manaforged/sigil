use super::super::rope::Rope;
use super::{Cursor, CursorSet};

impl CursorSet {
    pub fn add_next_match(&mut self, rope: &Rope, pattern: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }
        let after = self
            .cursors()
            .iter()
            .map(|c| c.span().end)
            .max()
            .unwrap_or(0);
        find_literal(rope, pattern, after).is_some_and(|start| {
            self.add(Cursor::selection(start, start + pattern.len()));
            self.coalesce();
            true
        })
    }
}

fn find_literal(rope: &Rope, pattern: &str, from: usize) -> Option<usize> {
    let text = rope.to_string_lossy();
    let start = from.min(text.len());
    text.get(start..)
        .and_then(|tail| tail.find(pattern))
        .map(|rel| start + rel)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "search_tests.rs"]
mod tests;
