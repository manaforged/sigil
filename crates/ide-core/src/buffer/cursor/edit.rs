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
#[path = "edit_tests.rs"]
mod tests;
