#![allow(clippy::redundant_pub_crate)]

const MAX_HISTORY: usize = 100;
const PAGE_LINES: isize = 10;

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
        match sigil_bidi::visual_step(&self.buffer, self.caret, right) {
            sigil_bidi::Step::To(pos) => Some(pos),
            sigil_bidi::Step::Edge => None,
            sigil_bidi::Step::Logical => logical(),
        }
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

    pub(crate) fn move_up(&mut self) {
        self.selection = None;
        self.caret = vertical_target(&self.buffer, self.caret, -1);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_down(&mut self) {
        self.selection = None;
        self.caret = vertical_target(&self.buffer, self.caret, 1);
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_up(&mut self) {
        self.selection.get_or_insert(self.caret);
        self.caret = vertical_target(&self.buffer, self.caret, -1);
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_down(&mut self) {
        self.selection.get_or_insert(self.caret);
        self.caret = vertical_target(&self.buffer, self.caret, 1);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_page_up(&mut self) {
        self.selection = None;
        self.caret = vertical_target(&self.buffer, self.caret, -PAGE_LINES);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_page_down(&mut self) {
        self.selection = None;
        self.caret = vertical_target(&self.buffer, self.caret, PAGE_LINES);
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_page_up(&mut self) {
        self.selection.get_or_insert(self.caret);
        self.caret = vertical_target(&self.buffer, self.caret, -PAGE_LINES);
        self.debug_assert_invariants();
    }

    pub(crate) fn shift_page_down(&mut self) {
        self.selection.get_or_insert(self.caret);
        self.caret = vertical_target(&self.buffer, self.caret, PAGE_LINES);
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

fn clamp_to_len_boundary(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while byte > 0 && !text.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
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

fn vertical_target(buffer: &str, caret: usize, dir: isize) -> usize {
    let mut starts = vec![0_usize];
    for (index, byte) in buffer.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    let line_index = starts.partition_point(|start| *start <= caret) - 1;
    let target_index = line_index.cast_signed() + dir;
    if target_index < 0 {
        return 0;
    }
    let Ok(target_index) = usize::try_from(target_index) else {
        return 0;
    };
    let Some(&target_start) = starts.get(target_index) else {
        return buffer.len();
    };
    let column = caret - starts.get(line_index).copied().unwrap_or(0);
    let line_end = starts
        .get(target_index + 1)
        .map_or(buffer.len(), |end| end.saturating_sub(1));
    let target = (target_start + column).min(line_end);
    super::floor_char_boundary(buffer, target)
}
