#![allow(clippy::redundant_pub_crate)]
#![allow(dead_code)]

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

    /// Clear the selection without moving the caret.
    pub(crate) const fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Set selection from a pointer click (sets anchor = caret = `pos`).
    /// `pos` is a byte offset clamped to the nearest char boundary.
    pub(crate) fn click_at(&mut self, pos: usize) {
        self.caret = clamp_to_boundary(&self.buffer, pos);
        self.selection = None;
        self.debug_assert_invariants();
    }

    /// Extend selection from the anchor to `pos` (drag motion).
    pub(crate) fn drag_to(&mut self, pos: usize) {
        self.caret = clamp_to_boundary(&self.buffer, pos);
        self.debug_assert_invariants();
    }

    /// Begin a drag: set anchor at current caret, then move caret to `pos`.
    pub(crate) fn begin_drag(&mut self, pos: usize) {
        let anchor = self.caret;
        self.caret = clamp_to_boundary(&self.buffer, pos);
        self.selection = if self.caret == anchor {
            None
        } else {
            Some(anchor)
        };
        self.debug_assert_invariants();
    }

    #[must_use]
    pub(crate) const fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.buffer.len()
    }

    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    #[must_use]
    pub(crate) const fn caret(&self) -> usize {
        self.caret
    }

    #[must_use]
    pub(crate) const fn selection_anchor(&self) -> Option<usize> {
        self.selection
    }

    #[must_use]
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

fn clamp_to_boundary(buffer: &str, pos: usize) -> usize {
    let clamped = pos.min(buffer.len());
    // Walk back to the nearest valid char boundary.
    (0..=clamped)
        .rev()
        .find(|&i| buffer.is_char_boundary(i))
        .unwrap_or(0)
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
