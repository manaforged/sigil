use std::ops::Range;

use super::super::rope::Rope;
use super::{Cursor, CursorSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Insert {
        at: usize,
        text: String,
    },
    Delete {
        from: usize,
        to: usize,
    },
    Replace {
        from: usize,
        to: usize,
        text: String,
    },
}

impl Edit {
    pub(crate) const fn range(&self) -> Range<usize> {
        match self {
            Self::Insert { at, .. } => *at..*at,
            Self::Delete { from, to } | Self::Replace { from, to, .. } => *from..*to,
        }
    }

    pub(crate) const fn new_len(&self) -> usize {
        match self {
            Self::Insert { text, .. } | Self::Replace { text, .. } => text.len(),
            Self::Delete { .. } => 0,
        }
    }
}

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
#[path = "edit_tests.rs"]
mod tests;
