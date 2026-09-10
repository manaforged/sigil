pub mod edit;
pub mod motion;
pub mod search;

use std::ops::Range;

use super::rope::Rope;

pub use edit::Edit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub anchor: usize,
    pub head: usize,
}

impl Cursor {
    #[must_use]
    pub const fn at(byte: usize) -> Self {
        Self {
            anchor: byte,
            head: byte,
        }
    }

    #[must_use]
    pub const fn selection(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    #[must_use]
    pub const fn span(&self) -> Range<usize> {
        if self.head >= self.anchor {
            self.anchor..self.head
        } else {
            self.head..self.anchor
        }
    }

    #[must_use]
    pub const fn is_caret(&self) -> bool {
        self.anchor == self.head
    }

    #[must_use]
    pub(crate) const fn is_forward(&self) -> bool {
        self.head >= self.anchor
    }
}

#[derive(Debug, Clone)]
pub struct CursorSet {
    cursors: Vec<Cursor>,
}

impl Default for CursorSet {
    fn default() -> Self {
        Self {
            cursors: vec![Cursor::at(0)],
        }
    }
}

impl CursorSet {
    #[must_use]
    pub fn new(initial: Cursor) -> Self {
        Self {
            cursors: vec![initial],
        }
    }

    #[must_use]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.cursors.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    #[must_use]
    pub fn primary(&self) -> Option<&Cursor> {
        self.cursors.first()
    }

    pub fn add(&mut self, c: Cursor) {
        let pos = self
            .cursors
            .partition_point(|existing| existing.head < c.head);
        self.cursors.insert(pos, c);
    }

    pub fn add_at(&mut self, byte: usize) {
        self.add(Cursor::at(byte));
    }

    pub fn set_primary(&mut self, c: Cursor) {
        self.cursors.clear();
        self.cursors.push(c);
    }

    pub fn move_each(&mut self, f: impl Fn(&Cursor) -> Cursor) {
        for c in &mut self.cursors {
            *c = f(c);
        }
        self.cursors.sort_by_key(|c| c.head);
        self.coalesce();
    }

    pub fn extend_each(&mut self, f: impl Fn(&Cursor) -> usize) {
        self.move_each(|c| Cursor {
            anchor: c.anchor,
            head: f(c),
        });
    }

    pub fn extend_to(&mut self, target_head: usize) {
        self.extend_each(|_| target_head);
    }

    pub fn coalesce(&mut self) {
        if self.cursors.len() < 2 {
            return;
        }
        self.cursors.sort_by_key(|c| (c.span().start, c.span().end));
        let mut merged: Vec<Cursor> = Vec::with_capacity(self.cursors.len());
        for c in self.cursors.drain(..) {
            match merged.last_mut() {
                Some(prev) if prev.span().end >= c.span().start => {
                    let start = prev.span().start.min(c.span().start);
                    let end = prev.span().end.max(c.span().end);
                    let forward = prev.is_forward() || c.is_forward();
                    *prev = if forward {
                        Cursor {
                            anchor: start,
                            head: end,
                        }
                    } else {
                        Cursor {
                            anchor: end,
                            head: start,
                        }
                    };
                }
                _ => merged.push(c),
            }
        }
        merged.sort_by_key(|c| c.head);
        self.cursors = merged;
    }

    pub fn edit<F>(&mut self, rope: &Rope, op: F) -> Rope
    where
        F: Fn(&Cursor, &Rope) -> Option<Edit>,
    {
        edit::run_edit(self, rope, op)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "cursor_tests.rs"]
mod tests;
