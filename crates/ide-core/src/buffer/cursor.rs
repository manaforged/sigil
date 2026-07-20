//! Multi-cursor primitives over the rope buffer.
//!
//! # Model
//!
//! A [`Cursor`] is an `anchor` + `head` pair of UTF-8 byte offsets into
//! a [`super::rope::Rope`]. When `anchor == head` the cursor is a caret;
//! otherwise it spans a selection `min(anchor, head)..max(anchor, head)`.
//!
//! A [`CursorSet`] owns a sorted-by-head `Vec<Cursor>` and provides:
//!
//! - Insertion ops ([`CursorSet::add`], [`CursorSet::add_at`],
//!   [`CursorSet::add_above`], [`CursorSet::add_below`],
//!   [`CursorSet::add_next_match`]).
//! - Motion ops ([`CursorSet::move_horizontal`],
//!   [`CursorSet::move_vertical`], [`CursorSet::move_to_line_start`],
//!   etc.).
//! - Selection extension ([`CursorSet::extend_to`]).
//! - Batched-edit primitive ([`CursorSet::edit`]) that produces a new
//!   rope per batch and remaps every cursor.
//!
//! # Edit ordering and offset remap
//!
//! [`CursorSet::edit`] collects an [`Edit`] per cursor (via the caller's
//! closure), then applies them to the rope in **descending span order**
//! so earlier edits don't shift the offsets of later ones. After each
//! edit lands, every other cursor's `head`/`anchor` is re-mapped using
//! `edit::remap_offset`.
//!
//! # Overlap and coalescing
//!
//! Cursors whose spans touch or overlap are merged by
//! [`CursorSet::coalesce`] into a single cursor covering the union of
//! their spans. Direction is preserved: if any contributing cursor had
//! `head > anchor`, the merged cursor has `head > anchor`. `edit` calls
//! `coalesce` before producing edits, so two cursors both deleting an
//! overlapping range produce one delete, not two.
//!
//! # Grapheme awareness
//!
//! Motion ops walk UTF-8 char boundaries (rope offsets are always char
//! boundaries; the rope panics on non-boundary inserts in debug). CRLF
//! pairs are treated atomically so left/right motion doesn't split
//! `\r\n` into two stops. Full grapheme-cluster motion (combining marks,
//! ZWJ sequences, regional indicators) is a follow-up that needs
//! `unicode-segmentation`; today motion stops are char boundaries.

pub mod edit;
pub mod motion;
pub mod search;

use std::ops::Range;

use super::rope::Rope;

pub use edit::Edit;

/// A single text cursor — anchor + active head, both UTF-8 byte offsets.
///
/// When `anchor == head` the cursor is a caret (no selection). Otherwise
/// the cursor spans `min(anchor, head)..max(anchor, head)`; `head > anchor`
/// means a forward selection, `head < anchor` a backward one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    /// Anchor byte offset.
    pub anchor: usize,
    /// Active byte offset. Equal to anchor means no selection.
    pub head: usize,
}

impl Cursor {
    /// Caret cursor at `byte` (anchor == head).
    #[must_use]
    pub const fn at(byte: usize) -> Self {
        Self {
            anchor: byte,
            head: byte,
        }
    }

    /// Selection cursor with explicit anchor + head.
    #[must_use]
    pub const fn selection(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Span as a byte range `min(anchor, head)..max(anchor, head)`.
    #[must_use]
    pub const fn span(&self) -> Range<usize> {
        if self.head >= self.anchor {
            self.anchor..self.head
        } else {
            self.head..self.anchor
        }
    }

    /// Whether `anchor == head`.
    #[must_use]
    pub const fn is_caret(&self) -> bool {
        self.anchor == self.head
    }

    /// Whether `head >= anchor` (forward direction or caret).
    #[must_use]
    pub(crate) const fn is_forward(&self) -> bool {
        self.head >= self.anchor
    }
}

/// An ordered, coalesced set of cursors over a single rope buffer.
///
/// Maintains the invariant that cursors are sorted by `head` (ascending)
/// and that no two spans overlap. Public mutators preserve the invariant;
/// after a [`CursorSet::move_each`] callback the set is re-sorted and
/// coalesced.
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
    /// Create a set seeded with one cursor.
    #[must_use]
    pub fn new(initial: Cursor) -> Self {
        Self {
            cursors: vec![initial],
        }
    }

    /// Current cursors in head-ascending order.
    #[must_use]
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Number of cursors in the set.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// The primary cursor — the first one in head-order. Returns
    /// `None` only for an empty set (which a `CursorSet` normally
    /// never is; `Default::default()` seeds one at offset 0).
    #[must_use]
    pub fn primary(&self) -> Option<&Cursor> {
        self.cursors.first()
    }

    /// Add a cursor, preserving the head-ascending sort order.
    ///
    /// Does NOT coalesce; call [`Self::coalesce`] to merge overlaps.
    pub fn add(&mut self, c: Cursor) {
        let pos = self
            .cursors
            .partition_point(|existing| existing.head < c.head);
        self.cursors.insert(pos, c);
    }

    /// Add a caret cursor at `byte`. Convenience over [`Self::add`].
    pub fn add_at(&mut self, byte: usize) {
        self.add(Cursor::at(byte));
    }

    /// Replace the set with one cursor.
    pub fn set_primary(&mut self, c: Cursor) {
        self.cursors.clear();
        self.cursors.push(c);
    }

    /// Apply `f` to each cursor, replacing it with the returned cursor,
    /// then re-sort by head and coalesce.
    pub fn move_each(&mut self, f: impl Fn(&Cursor) -> Cursor) {
        for c in &mut self.cursors {
            *c = f(c);
        }
        self.cursors.sort_by_key(|c| c.head);
        self.coalesce();
    }

    /// Extend every cursor's selection — anchor stays, head moves to
    /// `target_head` returned by `f`.
    pub fn extend_each(&mut self, f: impl Fn(&Cursor) -> usize) {
        self.move_each(|c| Cursor {
            anchor: c.anchor,
            head: f(c),
        });
    }

    /// Set every cursor's head to the result of `f`, preserving
    /// the cursor's anchor (selection-extension semantics).
    pub fn extend_to(&mut self, target_head: usize) {
        self.extend_each(|_| target_head);
    }

    /// Merge cursors whose spans touch or overlap.
    ///
    /// Two spans `a` and `b` are merged when `a.end >= b.start` (after
    /// sorting by span start). The merged cursor covers the union of
    /// the two spans; its direction is forward if any contributing
    /// cursor was forward (including pure carets).
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

    /// Apply an edit-producing closure to every cursor in a single
    /// rope checkpoint. See [`Edit`] and the module docs for the
    /// ordering + remap contract.
    pub fn edit<F>(&mut self, rope: &Rope, op: F) -> Rope
    where
        F: Fn(&Cursor, &Rope) -> Option<Edit>,
    {
        edit::run_edit(self, rope, op)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn text(rope: &Rope) -> String {
        rope.to_string_lossy()
    }

    #[test]
    fn single_cursor_insert() {
        let rope = Rope::from_str("hello world");
        let mut set = CursorSet::new(Cursor::at(5));
        let new_rope = set.edit(&rope, |c, _| {
            Some(Edit::Insert {
                at: c.head,
                text: ",".into(),
            })
        });
        assert_eq!(text(&new_rope), "hello, world");
        assert_eq!(set.cursors()[0].head, 6);
    }

    #[test]
    fn single_cursor_delete() {
        let rope = Rope::from_str("hello world");
        let mut set = CursorSet::new(Cursor::selection(0, 6));
        let new_rope = set.edit(&rope, |c, _| {
            let s = c.span();
            Some(Edit::Delete {
                from: s.start,
                to: s.end,
            })
        });
        assert_eq!(text(&new_rope), "world");
        assert_eq!(set.cursors()[0].head, 0);
        assert!(set.cursors()[0].is_caret());
    }

    #[test]
    fn add_preserves_sort_order() {
        let mut set = CursorSet::new(Cursor::at(10));
        set.add(Cursor::at(2));
        set.add(Cursor::at(7));
        set.add(Cursor::at(0));
        set.add(Cursor::at(15));
        let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
        assert_eq!(heads, vec![0, 2, 7, 10, 15]);
    }

    #[test]
    fn overlapping_selections_coalesce() {
        let mut set = CursorSet::new(Cursor::selection(5, 10));
        set.add(Cursor::selection(8, 12));
        set.coalesce();
        assert_eq!(set.cursors().len(), 1);
        assert_eq!(set.cursors()[0].span(), 5..12);
    }

    #[test]
    fn touching_selections_coalesce() {
        let mut set = CursorSet::new(Cursor::selection(0, 5));
        set.add(Cursor::selection(5, 10));
        set.coalesce();
        assert_eq!(set.cursors().len(), 1);
        assert_eq!(set.cursors()[0].span(), 0..10);
    }

    #[test]
    fn contained_selection_coalesce() {
        let mut set = CursorSet::new(Cursor::selection(2, 10));
        set.add(Cursor::selection(4, 6));
        set.coalesce();
        assert_eq!(set.cursors().len(), 1);
        assert_eq!(set.cursors()[0].span(), 2..10);
    }

    #[test]
    fn coalesce_preserves_forward_direction() {
        // Two cursors, one forward, one backward — merged is forward.
        let mut set = CursorSet::new(Cursor::selection(0, 5));
        set.add(Cursor::selection(8, 4));
        set.coalesce();
        assert_eq!(set.cursors().len(), 1);
        let c = set.cursors()[0];
        assert_eq!(c.span(), 0..8);
        assert!(
            c.is_forward(),
            "merged should be forward when any input was"
        );
    }

    #[test]
    fn extend_to_preserves_anchor() {
        let mut set = CursorSet::new(Cursor::at(5));
        set.extend_to(10);
        let c = set.cursors()[0];
        assert_eq!(c.anchor, 5);
        assert_eq!(c.head, 10);
    }

    #[test]
    fn set_primary_replaces() {
        let mut set = CursorSet::new(Cursor::at(0));
        set.add_at(5);
        set.add_at(10);
        set.set_primary(Cursor::at(42));
        assert_eq!(set.cursors().len(), 1);
        assert_eq!(set.cursors()[0].head, 42);
    }

    #[test]
    fn primary_returns_first() {
        let mut set = CursorSet::new(Cursor::at(10));
        set.add(Cursor::at(2));
        let p = set.primary().expect("at least one cursor");
        assert_eq!(p.head, 2);
    }

    #[test]
    fn default_is_caret_at_zero() {
        let set = CursorSet::default();
        assert_eq!(set.len(), 1);
        assert_eq!(set.cursors()[0], Cursor::at(0));
    }
}
