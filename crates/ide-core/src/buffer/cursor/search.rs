//! Literal-match "next-occurrence" cursor insertion.
//!
//! Sublime/VSCode-style "add cursor at next match". We walk the rope's
//! collected text for an exact byte-level substring match starting
//! AFTER the existing cursor with the highest head position; on match,
//! we add a selection cursor `match_start..match_end` and coalesce.
//!
//! Why not the I-T5 fuzzy matcher: "next match" in the editor sense
//! is a literal substring scan (Vim's `*`, `VSCode` Ctrl-D). Fuzzy is
//! for the command palette, not the cursor-multiplier hotkey.

use super::super::rope::Rope;
use super::{Cursor, CursorSet};

impl CursorSet {
    /// Add a cursor selection at the next literal occurrence of
    /// `pattern` after the last cursor's head. Returns `true` if a
    /// match was found and added (or coalesced).
    ///
    /// An empty `pattern` is a no-op (returns `false`).
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

/// Find the byte offset of the next literal match of `pattern` in
/// `rope` starting at or after `from`. Returns `None` if no match.
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
