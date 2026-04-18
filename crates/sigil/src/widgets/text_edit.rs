#![allow(clippy::redundant_pub_crate)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextEdit {
    buffer: String,
    caret: usize,
}

impl TextEdit {
    #[must_use]
    pub(crate) fn new(buffer: String) -> Self {
        let caret = buffer.len();
        let this = Self { buffer, caret };
        this.debug_assert_invariants();
        this
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.buffer.insert_str(self.caret, text);
        self.caret += text.len();
        self.debug_assert_invariants();
    }

    pub(crate) fn backspace(&mut self) {
        let Some(previous) = previous_boundary(&self.buffer, self.caret) else {
            return;
        };
        self.buffer.drain(previous..self.caret);
        self.caret = previous;
        self.debug_assert_invariants();
    }

    pub(crate) fn delete_forward(&mut self) {
        let Some(next) = next_boundary(&self.buffer, self.caret) else {
            return;
        };
        self.buffer.drain(self.caret..next);
        self.debug_assert_invariants();
    }

    pub(crate) fn move_left(&mut self) {
        if let Some(previous) = previous_boundary(&self.buffer, self.caret) {
            self.caret = previous;
        }
        self.debug_assert_invariants();
    }

    pub(crate) fn move_right(&mut self) {
        if let Some(next) = next_boundary(&self.buffer, self.caret) {
            self.caret = next;
        }
        self.debug_assert_invariants();
    }

    pub(crate) const fn move_home(&mut self) {
        self.caret = 0;
    }

    pub(crate) fn move_end(&mut self) {
        self.caret = self.buffer.len();
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

    #[must_use]
    pub(crate) const fn caret(&self) -> usize {
        self.caret
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.buffer
    }

    fn debug_assert_invariants(&self) {
        debug_assert!(self.caret <= self.buffer.len());
        debug_assert!(self.buffer.is_char_boundary(self.caret));
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
