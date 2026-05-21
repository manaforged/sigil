//! Batched edit application + offset remap for [`super::CursorSet`].
//!
//! The contract: take one [`Edit`] per cursor (or skip via `None`),
//! apply in descending span-start order on a clone of the rope, and
//! remap every cursor's `head`/`anchor` after each apply so the set
//! ends up referring to the right positions in the new rope.

use std::ops::Range;

use super::super::rope::Rope;
use super::{Cursor, CursorSet};

/// A single batched edit operation on a rope.
///
/// Returned by the closure passed to [`super::CursorSet::edit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Insert `text` at byte offset `at`.
    Insert {
        /// Byte offset (must be a UTF-8 char boundary).
        at: usize,
        /// Text to insert.
        text: String,
    },
    /// Delete bytes in `from..to`.
    Delete {
        /// Inclusive start byte offset.
        from: usize,
        /// Exclusive end byte offset.
        to: usize,
    },
    /// Replace bytes in `from..to` with `text`.
    Replace {
        /// Inclusive start byte offset.
        from: usize,
        /// Exclusive end byte offset.
        to: usize,
        /// Replacement text.
        text: String,
    },
}

impl Edit {
    /// Byte range this edit will mutate in the source rope.
    pub(crate) const fn range(&self) -> Range<usize> {
        match self {
            Self::Insert { at, .. } => *at..*at,
            Self::Delete { from, to } | Self::Replace { from, to, .. } => *from..*to,
        }
    }

    /// New length of the inserted/replaced text (0 for delete).
    pub(crate) const fn new_len(&self) -> usize {
        match self {
            Self::Insert { text, .. } | Self::Replace { text, .. } => text.len(),
            Self::Delete { .. } => 0,
        }
    }
}

/// Implement [`super::CursorSet::edit`]. Defined here so the module
/// root stays small.
pub(crate) fn run_edit<F>(set: &mut CursorSet, rope: &Rope, op: F) -> Rope
where
    F: Fn(&Cursor, &Rope) -> Option<Edit>,
{
    set.coalesce();
    let edits: Vec<Edit> = set.cursors.iter().filter_map(|c| op(c, rope)).collect();
    if edits.is_empty() {
        return rope.clone();
    }

    let mut sorted = edits;
    sorted.sort_by_key(|e| std::cmp::Reverse(e.range().start));

    let mut new_rope = rope.clone();
    for edit in &sorted {
        apply_edit(&mut new_rope, edit);
        for c in &mut set.cursors {
            c.anchor = remap_offset(c.anchor, edit);
            c.head = remap_offset(c.head, edit);
        }
    }

    set.cursors.sort_by_key(|c| c.head);
    set.coalesce();
    new_rope
}

fn apply_edit(rope: &mut Rope, edit: &Edit) {
    match edit {
        Edit::Insert { at, text } => rope.insert(*at, text),
        Edit::Delete { from, to } => rope.remove(*from..*to),
        Edit::Replace { from, to, text } => {
            rope.remove(*from..*to);
            rope.insert(*from, text);
        }
    }
}

/// Re-map a byte offset after `edit` has been applied to the rope.
///
/// Offsets strictly before the edit's start are unchanged. Offsets inside
/// the edit's deleted range collapse to `start + new_len`. Offsets at or
/// after the edit's end shift by the signed length delta `new_len -
/// old_len`. For a pure [`Edit::Insert`] (empty range), offsets equal
/// to the insertion point shift forward by `new_len` (sticky-right
/// gravity): the cursor that performed the insert ends up after the
/// inserted text.
pub(crate) const fn remap_offset(offset: usize, edit: &Edit) -> usize {
    let range = edit.range();
    let new_len = edit.new_len();
    let old_len = range.end - range.start;
    if offset < range.start {
        offset
    } else if offset < range.end {
        range.start + new_len
    } else {
        offset + new_len - old_len
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::super::Cursor;
    use super::*;

    fn text(rope: &Rope) -> String {
        rope.to_string_lossy()
    }

    #[test]
    fn three_cursors_parallel_insert_remap_correct() {
        // Insert 'X' at offsets 2, 5, 8 of "abcdefghij" → "abXcdeXfghXij".
        // Verify each cursor lands AFTER its own insert (sticky-right).
        let rope = Rope::from_str("abcdefghij");
        let mut set = CursorSet::new(Cursor::at(2));
        set.add(Cursor::at(5));
        set.add(Cursor::at(8));
        let new_rope = set.edit(&rope, |c, _| {
            Some(Edit::Insert {
                at: c.head,
                text: "X".into(),
            })
        });
        assert_eq!(text(&new_rope), "abXcdeXfghXij");
        let heads: Vec<usize> = set.cursors().iter().map(|c| c.head).collect();
        // Cursor at 2: inserted X at 2 → 3. No other edits before it.
        // Cursor at 5: own insert pushes to 6, then insert-at-2 (before it) +1 → 7.
        // Cursor at 8: own insert pushes to 9, then insert-at-5 +1 → 10, then insert-at-2 +1 → 11.
        assert_eq!(heads, vec![3, 7, 11]);
    }

    #[test]
    fn parallel_delete_remap_correct() {
        // Delete [1..3) and [6..8) of "abcdefghij" → "adefij".
        let rope = Rope::from_str("abcdefghij");
        let mut set = CursorSet::new(Cursor::selection(1, 3));
        set.add(Cursor::selection(6, 8));
        let new_rope = set.edit(&rope, |c, _| {
            let s = c.span();
            Some(Edit::Delete {
                from: s.start,
                to: s.end,
            })
        });
        assert_eq!(text(&new_rope), "adefij");
        let cursors = set.cursors();
        assert_eq!(cursors[0].head, 1);
        assert_eq!(cursors[0].anchor, 1);
        // Second cursor: its own span collapses to start=6, then two bytes
        // deleted before it shift -2 → 4.
        assert_eq!(cursors[1].head, 4);
    }

    #[test]
    fn overlapping_delete_produces_one_edit() {
        // [2..6) and [4..8) coalesce to [2..8) → one delete of "cdef gh".
        let rope = Rope::from_str("abcdefghij");
        let mut set = CursorSet::new(Cursor::selection(2, 6));
        set.add(Cursor::selection(4, 8));
        let new_rope = set.edit(&rope, |c, _| {
            let s = c.span();
            Some(Edit::Delete {
                from: s.start,
                to: s.end,
            })
        });
        assert_eq!(text(&new_rope), "abij");
        assert_eq!(set.cursors().len(), 1);
    }

    #[test]
    fn replace_remaps_trailing_cursors() {
        // Replace "world" (6..11) with "earth!" (+1 byte); the cursor at 17 must shift to 18.
        let rope = Rope::from_str("hello world today");
        let mut set = CursorSet::new(Cursor::at(0));
        set.add(Cursor::selection(6, 11));
        set.add(Cursor::at(17));
        let new_rope = set.edit(&rope, |c, _| {
            if c.span().start == 6 {
                Some(Edit::Replace {
                    from: 6,
                    to: 11,
                    text: "earth!".into(),
                })
            } else {
                None
            }
        });
        assert_eq!(text(&new_rope), "hello earth! today");
        let cursors = set.cursors();
        assert_eq!(cursors[0].head, 0);
        assert_eq!(cursors[1].head, 12);
        assert_eq!(cursors[2].head, 18);
    }

    #[test]
    fn noop_edit_returns_clone_of_original() {
        let rope = Rope::from_str("hello world");
        let mut set = CursorSet::new(Cursor::at(5));
        set.add(Cursor::at(11));
        let new_rope = set.edit(&rope, |_, _| None);
        assert_eq!(text(&new_rope), "hello world");
        assert_eq!(set.cursors().len(), 2);
    }

    #[test]
    fn insert_at_buffer_end_works() {
        let rope = Rope::from_str("abc");
        let mut set = CursorSet::new(Cursor::at(3));
        let new_rope = set.edit(&rope, |c, _| {
            Some(Edit::Insert {
                at: c.head,
                text: "!".into(),
            })
        });
        assert_eq!(text(&new_rope), "abc!");
        assert_eq!(set.cursors()[0].head, 4);
    }
}
