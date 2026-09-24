#![allow(clippy::redundant_pub_crate)]

use unicode_segmentation::UnicodeSegmentation;

use crate::widgets::grapheme::{
    clamp_to_len_boundary, line_end, line_start, next_boundary, previous_boundary, selection_range,
    word_end_after, word_start_before,
};

const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextEdit {
    buffer: String,
    caret: usize,
    selection: Option<usize>,
    history: Vec<Snapshot>,
    future: Vec<Snapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    buffer: String,
    caret: usize,
}

impl TextEdit {
    #[must_use]
    pub(crate) fn new_at(buffer: String, caret: usize) -> Self {
        let caret = clamp_to_len_boundary(&buffer, caret);
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

    pub(crate) fn replace(&mut self, text: &str) {
        if self.buffer == text {
            return;
        }
        self.push_history();
        self.buffer = text.to_string();
        self.caret = self.buffer.len();
        self.selection = None;
        self.debug_assert_invariants();
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.push_history();
        let position = self
            .selection
            .map_or(self.caret, |anchor| anchor.min(self.caret));
        self.delete_selection_range();
        self.buffer.insert_str(position, text);
        self.caret = self
            .buffer
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .find(|index| *index >= position + text.len())
            .unwrap_or(self.buffer.len());
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
        self.caret = self.clamp_boundary(previous);
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
        self.caret = self.clamp_boundary(self.caret);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_left(&mut self) {
        self.selection = None;
        if let Some(next) = self.step(false) {
            self.caret = next;
        }
        self.debug_assert_invariants();
    }

    pub(crate) fn move_right(&mut self) {
        self.selection = None;
        if let Some(next) = self.step(true) {
            self.caret = next;
        }
        self.debug_assert_invariants();
    }

    fn step(&self, right: bool) -> Option<usize> {
        let logical = || {
            if right {
                next_boundary(&self.buffer, self.caret)
            } else {
                previous_boundary(&self.buffer, self.caret)
            }
        };
        if self.buffer.contains('\n') {
            return logical();
        }
        match crate::bidi::visual_step(&self.buffer, self.caret, right) {
            crate::bidi::Step::To(pos) if pos > self.caret => {
                next_boundary(&self.buffer, self.caret)
            }
            crate::bidi::Step::To(_) => previous_boundary(&self.buffer, self.caret),
            crate::bidi::Step::Edge => None,
            crate::bidi::Step::Logical => logical(),
        }
    }

    pub(crate) fn move_word_left(&mut self) {
        self.selection = None;
        self.caret = word_start_before(&self.buffer, self.caret);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_word_right(&mut self) {
        self.selection = None;
        self.caret = word_end_after(&self.buffer, self.caret);
        self.debug_assert_invariants();
    }

    pub(crate) fn delete_word_backward(&mut self) {
        self.delete_span(word_start_before(&self.buffer, self.caret), self.caret);
    }

    pub(crate) fn delete_word_forward(&mut self) {
        self.delete_span(self.caret, word_end_after(&self.buffer, self.caret));
    }

    pub(crate) fn delete_to_line_start(&mut self) {
        self.delete_span(line_start(&self.buffer, self.caret), self.caret);
    }

    pub(crate) fn delete_to_line_end(&mut self) {
        self.delete_span(self.caret, line_end(&self.buffer, self.caret));
    }

    fn delete_span(&mut self, start: usize, end: usize) {
        if self.selection.is_some() {
            self.push_history();
            self.delete_selection_range();
            return;
        }
        if start >= end {
            return;
        }
        self.push_history();
        self.buffer.drain(start..end);
        self.caret = self.clamp_boundary(start);
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

    pub(crate) fn shift_left(&mut self) {
        let anchor = *self.selection.get_or_insert(self.caret);
        if let Some(previous) = self.step(false) {
            self.caret = previous;
        }
        if self.caret == anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_right(&mut self) {
        let anchor = *self.selection.get_or_insert(self.caret);
        if let Some(next) = self.step(true) {
            self.caret = next;
        }
        if self.caret == anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_home(&mut self) {
        if self.caret == 0 {
            return;
        }
        self.selection.get_or_insert(self.caret);
        self.caret = 0;
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_end(&mut self) {
        let end = self.buffer.len();
        if self.caret == end {
            return;
        }
        self.selection.get_or_insert(self.caret);
        self.caret = end;
        self.debug_assert_invariants();
    }

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

    #[must_use]
    pub(crate) fn selected_text(&self) -> Option<&str> {
        let anchor = self.selection?;
        let (start, end) = selection_range(anchor, self.caret);
        Some(&self.buffer[start..end])
    }

    pub(crate) const fn select_all(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        self.selection = Some(0);
        self.caret = self.buffer.len();
    }

    pub(crate) fn place_caret(&mut self, byte: usize) {
        self.caret = self.clamp_boundary(byte);
        self.selection = None;
        self.debug_assert_invariants();
    }

    pub(crate) fn extend_to(&mut self, byte: usize) {
        let byte = self.clamp_boundary(byte);
        let anchor = *self.selection.get_or_insert(self.caret);
        self.caret = byte;
        if self.caret == anchor {
            self.selection = None;
        }
        self.debug_assert_invariants();
    }

    #[must_use]
    pub(crate) const fn selection(&self) -> Option<usize> {
        self.selection
    }

    fn clamp_boundary(&self, byte: usize) -> usize {
        clamp_to_len_boundary(&self.buffer, byte)
    }

    pub(crate) fn sync_external(&mut self, external: String) {
        if self.buffer == external {
            return;
        }
        self.caret = clamp_to_len_boundary(&external, self.caret);
        self.selection = self
            .selection
            .map(|anchor| clamp_to_len_boundary(&external, anchor));
        self.buffer = external;
        self.debug_assert_invariants();
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

    fn delete_selection_range(&mut self) {
        let Some(anchor) = self.selection.take() else {
            return;
        };
        let (start, end) = selection_range(anchor, self.caret);
        self.buffer.drain(start..end);
        self.caret = self.clamp_boundary(start);
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
