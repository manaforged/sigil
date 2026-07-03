//! Internal rope/buffer + caret + selection + undo state.
//!
//! Shared editing primitive used by both [`text_field`] (single-line)
//! and [`text_edit`] (multi-line) widgets. The struct deliberately does
//! NOT know whether `\n` is allowed — callers gate input semantics.
//!
//! [`text_field`]: super::super::text_field
//! [`text_edit`]: super

#![allow(clippy::redundant_pub_crate)]

const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextEdit {
    buffer: String,
    caret: usize,
    /// Anchor byte offset for selection. When `Some(anchor)`, the selection
    /// spans `anchor..caret` or `caret..anchor`. `None` means caret-only.
    selection: Option<usize>,
    history: Vec<Snapshot>,
    future: Vec<Snapshot>,
}

/// A point-in-time snapshot of buffer + caret used by undo/redo.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    buffer: String,
    caret: usize,
}

impl TextEdit {
    #[must_use]
    pub(crate) fn new(buffer: String) -> Self {
        let caret = buffer.len();
        let this = Self {
            buffer,
            caret,
            selection: None,
            history: Vec::new(),
            future: Vec::new(),
        };
        this.debug_assert_invariants();
        this
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.push_history();
        self.delete_selection_range();
        self.buffer.insert_str(self.caret, text);
        self.caret += text.len();
        self.selection = None;
        self.debug_assert_invariants();
    }

    pub(crate) fn backspace(&mut self) {
        if self.selection.is_some() {
            self.push_history();
            self.delete_selection_range();
            return;
        }
        let Some(previous) = previous_boundary(&self.buffer, self.caret) else {
            return;
        };
        self.push_history();
        self.buffer.drain(previous..self.caret);
        self.caret = previous;
        self.debug_assert_invariants();
    }

    pub(crate) fn delete_forward(&mut self) {
        if self.selection.is_some() {
            self.push_history();
            self.delete_selection_range();
            return;
        }
        let Some(next) = next_boundary(&self.buffer, self.caret) else {
            return;
        };
        self.push_history();
        self.buffer.drain(self.caret..next);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_left(&mut self) {
        self.selection = None;
        if let Some(previous) = previous_boundary(&self.buffer, self.caret) {
            self.caret = previous;
        }
        self.debug_assert_invariants();
    }

    pub(crate) fn move_right(&mut self) {
        self.selection = None;
        if let Some(next) = next_boundary(&self.buffer, self.caret) {
            self.caret = next;
        }
        self.debug_assert_invariants();
    }

    pub(crate) const fn move_home(&mut self) {
        self.selection = None;
        self.caret = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.selection = None;
        self.caret = self.buffer.len();
        self.debug_assert_invariants();
    }

    /// Extend selection one character to the left.
    pub(crate) fn shift_left(&mut self) {
        let anchor = self.selection.get_or_insert(self.caret);
        if let Some(previous) = previous_boundary(&self.buffer, self.caret) {
            self.caret = previous;
        }
        if self.caret == *anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    /// Extend selection one character to the right.
    pub(crate) fn shift_right(&mut self) {
        let anchor = self.selection.get_or_insert(self.caret);
        if let Some(next) = next_boundary(&self.buffer, self.caret) {
            self.caret = next;
        }
        if self.caret == *anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    /// Extend selection to the start of the buffer.
    pub(crate) fn shift_home(&mut self) {
        if self.caret == 0 {
            return;
        }
        self.selection.get_or_insert(self.caret);
        self.caret = 0;
        self.debug_assert_invariants();
    }

    /// Extend selection to the end of the buffer.
    pub(crate) fn shift_end(&mut self) {
        let end = self.buffer.len();
        if self.caret == end {
            return;
        }
        self.selection.get_or_insert(self.caret);
        self.caret = end;
        self.debug_assert_invariants();
    }

    /// Restore the previous edit state. Returns `true` if state changed.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(snapshot) = self.history.pop() else {
            return false;
        };
        let current = Snapshot {
            buffer: self.buffer.clone(),
            caret: self.caret,
        };
        if self.future.len() >= MAX_HISTORY {
            self.future.remove(0);
        }
        self.future.push(current);
        self.buffer = snapshot.buffer;
        self.caret = snapshot.caret;
        self.selection = None;
        self.debug_assert_invariants();
        true
    }

    /// Reapply the next edit state. Returns `true` if state changed.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(snapshot) = self.future.pop() else {
            return false;
        };
        let current = Snapshot {
            buffer: self.buffer.clone(),
            caret: self.caret,
        };
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(current);
        self.buffer = snapshot.buffer;
        self.caret = snapshot.caret;
        self.selection = None;
        self.debug_assert_invariants();
        true
    }

    /// Return the selected text slice, or `None` if the selection is empty.
    #[must_use]
    pub(crate) fn selected_text(&self) -> Option<&str> {
        let anchor = self.selection?;
        let (start, end) = selection_range(anchor, self.caret);
        Some(&self.buffer[start..end])
    }

    /// Select all text.
    pub(crate) const fn select_all(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        self.selection = Some(0);
        self.caret = self.buffer.len();
    }

    /// Place the caret at `byte`, collapsing any active selection.
    ///
    /// Used by pointer-down to set the caret at the click position. `byte` is
    /// clamped to a UTF-8 boundary at or below the buffer length, so callers
    /// may pass a raw hit-test result. Caret moves are not undoable, so this
    /// records no history (consistent with [`Self::move_left`] et al.).
    pub(crate) fn place_caret(&mut self, byte: usize) {
        self.caret = self.clamp_boundary(byte);
        self.selection = None;
        self.debug_assert_invariants();
    }

    /// Move the caret to `byte`, extending the selection from its anchor.
    ///
    /// Used by pointer-drag. The first call after [`Self::place_caret`] turns
    /// the current caret into the anchor; a drag that lands back on the anchor
    /// collapses the selection (mirrors [`Self::shift_left`]).
    pub(crate) fn extend_to(&mut self, byte: usize) {
        let byte = self.clamp_boundary(byte);
        let anchor = *self.selection.get_or_insert(self.caret);
        self.caret = byte;
        if self.caret == anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    /// Current selection anchor, or `None` when the caret is collapsed.
    #[must_use]
    pub(crate) const fn selection(&self) -> Option<usize> {
        self.selection
    }

    /// Clamp `byte` to the nearest UTF-8 boundary at or below the buffer length.
    fn clamp_boundary(&self, byte: usize) -> usize {
        let mut byte = byte.min(self.buffer.len());
        while byte > 0 && !self.buffer.is_char_boundary(byte) {
            byte -= 1;
        }
        byte
    }

    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.buffer.len()
    }

    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub(crate) const fn caret(&self) -> usize {
        self.caret
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Delete the selected range and collapse caret to the start.
    fn delete_selection_range(&mut self) {
        let Some(anchor) = self.selection.take() else {
            return;
        };
        let (start, end) = selection_range(anchor, self.caret);
        self.buffer.drain(start..end);
        self.caret = start;
        self.debug_assert_invariants();
    }

    fn push_history(&mut self) {
        let snapshot = Snapshot {
            buffer: self.buffer.clone(),
            caret: self.caret,
        };
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(snapshot);
        self.future.clear();
    }

    fn debug_assert_invariants(&self) {
        debug_assert!(self.caret <= self.buffer.len());
        debug_assert!(self.buffer.is_char_boundary(self.caret));
        if let Some(anchor) = self.selection {
            debug_assert!(anchor <= self.buffer.len());
            debug_assert!(self.buffer.is_char_boundary(anchor));
        }
    }
}

const fn selection_range(anchor: usize, caret: usize) -> (usize, usize) {
    if anchor < caret {
        (anchor, caret)
    } else {
        (caret, anchor)
    }
}

fn previous_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret == 0 {
        return None;
    }
    buffer[..caret]
        .char_indices()
        .last()
        .map(|(index, _)| index)
}

fn next_boundary(buffer: &str, caret: usize) -> Option<usize> {
    if caret >= buffer.len() {
        return None;
    }
    buffer[caret..]
        .char_indices()
        .nth(1)
        .map_or(Some(buffer.len()), |(index, _)| Some(caret + index))
}

#[cfg(test)]
mod tests {
    use super::TextEdit;

    #[test]
    fn place_caret_sets_caret_and_collapses_selection() {
        let mut edit = TextEdit::new("hello world".to_owned());
        edit.select_all();
        assert!(edit.selection().is_some());
        edit.place_caret(3);
        assert_eq!(edit.caret(), 3);
        assert_eq!(edit.selection(), None, "click collapses the selection");
    }

    #[test]
    fn place_caret_clamps_past_end_and_off_boundary() {
        // "café" — the 'é' is 2 bytes (3..5), so byte 4 is mid-codepoint.
        let mut edit = TextEdit::new("café".to_owned());
        edit.place_caret(999);
        assert_eq!(edit.caret(), edit.len(), "past-end clamps to length");
        edit.place_caret(4);
        assert_eq!(edit.caret(), 3, "mid-codepoint snaps back to a boundary");
    }

    #[test]
    fn extend_to_anchors_at_starting_caret_then_grows() {
        let mut edit = TextEdit::new("hello world".to_owned());
        edit.place_caret(2); // caret at 2, no selection
        edit.extend_to(6); // drag right to byte 6 (exclusive)
        assert_eq!(edit.caret(), 6);
        assert_eq!(edit.selection(), Some(2), "anchor stays at the drag origin");
        assert_eq!(edit.selected_text(), Some("llo "));
    }

    #[test]
    fn extend_to_back_onto_anchor_collapses() {
        let mut edit = TextEdit::new("hello".to_owned());
        edit.place_caret(1);
        edit.extend_to(4);
        assert_eq!(edit.selection(), Some(1));
        edit.extend_to(1); // drag back to the origin
        assert_eq!(edit.selection(), None, "a zero-width drag collapses");
        assert_eq!(edit.caret(), 1);
    }
}
