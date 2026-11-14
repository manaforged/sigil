use std::cell::{Cell, RefCell, RefMut};
use std::ops::Range;
use std::rc::Rc;

use crate::reactive::{Signal, batch, signal};

use super::super::TextEdit;

/// Caret and undo state of a text editor.
#[derive(Clone)]
pub struct TextState {
    pub(in crate::widgets) value: Signal<String>,
    pub(in crate::widgets) caret: Signal<usize>,
    pub(super) selection: Signal<Option<usize>>,
    pub(in crate::widgets) edit: Rc<RefCell<TextEdit>>,
    pub(super) preferred_x: Rc<Cell<Option<f32>>>,
}

impl std::fmt::Debug for TextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextState")
            .field("value", &self.value)
            .field("caret", &self.caret)
            .field("selection", &self.selection)
            .finish_non_exhaustive()
    }
}

impl TextState {
    /// Creates a state over `value` with the caret at the byte offset in `caret`.
    #[must_use]
    pub fn new(value: Signal<String>, caret: Signal<usize>) -> Self {
        let edit = TextEdit::new_at(value.peek(), caret.peek());
        caret.set(edit.caret());
        Self {
            value,
            caret,
            selection: signal(None),
            edit: Rc::new(RefCell::new(edit)),
            preferred_x: Rc::new(Cell::new(None)),
        }
    }

    /// Returns the text signal.
    #[must_use]
    pub const fn value(&self) -> Signal<String> {
        self.value
    }

    /// Returns the caret byte-offset signal.
    #[must_use]
    pub const fn caret(&self) -> Signal<usize> {
        self.caret
    }

    pub(in crate::widgets) fn rebind_caret(&mut self, position: Signal<usize>) {
        self.caret = position;
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        edit.place_caret(position.peek());
        if position.peek() != edit.caret() {
            position.set(edit.caret());
        }
    }

    /// Places the caret at `byte` and clears the selection.
    pub fn select(&self, byte: usize) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.place_caret(byte);
        self.publish(&edit, false);
    }

    /// Extends the selection to `byte`.
    pub fn extend(&self, byte: usize) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.extend_to(byte);
        self.publish(&edit, false);
    }

    /// Clears the selection while keeping the caret at its current position.
    pub fn clear_selection(&self) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        let caret = edit.caret();
        edit.place_caret(caret);
        self.publish(&edit, false);
    }

    /// Inserts `text` at the caret.
    pub fn insert(&self, text: &str) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.insert_str(text);
        self.publish(&edit, true);
    }

    /// Replaces all the text.
    pub fn replace(&self, text: &str) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.replace(text);
        self.publish(&edit, true);
    }

    /// Deletes the selection or the grapheme before the caret.
    pub fn backspace(&self) {
        self.apply_edit(TextEdit::backspace);
    }

    /// Deletes the selection or the grapheme after the caret.
    pub fn delete_forward(&self) {
        self.apply_edit(TextEdit::delete_forward);
    }

    /// Moves the caret one grapheme horizontally, extending the selection when `extend` is true.
    pub fn move_by(&self, right: bool, extend: bool) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.move_by(right, extend);
        self.publish(&edit, false);
    }

    /// Moves the caret to the start of the previous word and clears the selection.
    pub fn move_word_left(&self) {
        self.apply_move(TextEdit::move_word_left);
    }

    /// Moves the caret to the end of the next word and clears the selection.
    pub fn move_word_right(&self) {
        self.apply_move(TextEdit::move_word_right);
    }

    /// Deletes the selection or from the previous word start to the caret.
    pub fn delete_word_backward(&self) {
        self.apply_edit(TextEdit::delete_word_backward);
    }

    /// Deletes the selection or from the caret to the next word end.
    pub fn delete_word_forward(&self) {
        self.apply_edit(TextEdit::delete_word_forward);
    }

    /// Selects all text.
    pub fn select_all(&self) {
        self.apply_move(TextEdit::select_all);
    }

    /// Undoes the previous edit and reports whether the text changed.
    pub fn undo(&self) -> bool {
        self.apply_history(TextEdit::undo)
    }

    /// Reapplies an undone edit and reports whether the text changed.
    pub fn redo(&self) -> bool {
        self.apply_history(TextEdit::redo)
    }

    /// Returns the selected text, or `None` when nothing is selected.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        let edit = synced(&self.edit, self.value, self.caret);
        edit.selected_text().map(str::to_owned)
    }

    /// Splices `preedit` into `text` at the char boundary at or before `caret`.
    #[must_use]
    pub fn splice_preedit(text: &str, caret: usize, preedit: &str) -> String {
        super::super::splice_preedit(text, caret, preedit)
    }

    pub(in crate::widgets) fn apply_edit(&self, op: fn(&mut TextEdit)) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        let before = edit.len();
        op(&mut edit);
        self.publish(&edit, edit.len() != before);
    }

    fn apply_move(&self, op: fn(&mut TextEdit)) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        op(&mut edit);
        self.publish(&edit, false);
    }

    fn apply_history(&self, op: fn(&mut TextEdit) -> bool) -> bool {
        let mut edit = synced(&self.edit, self.value, self.caret);
        let changed = op(&mut edit);
        if changed {
            self.publish(&edit, true);
        }
        changed
    }

    fn publish(&self, edit: &TextEdit, value: bool) {
        self.preferred_x.set(None);
        batch(|| {
            if value {
                super::write_value(self.value, edit.as_str());
            }
            if self.caret.peek() != edit.caret() {
                self.caret.set(edit.caret());
            }
            if self.selection.peek() != edit.selection() {
                self.selection.set(edit.selection());
            }
        });
    }

    /// The selected byte range; `None` when nothing is selected.
    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.selection.get()?;
        let caret = self.caret.get();
        Some(anchor.min(caret)..anchor.max(caret))
    }
}

impl From<Signal<String>> for TextState {
    fn from(value: Signal<String>) -> Self {
        Self::new(value, signal(value.peek().len()))
    }
}

impl From<&Self> for TextState {
    fn from(state: &Self) -> Self {
        state.clone()
    }
}

pub(super) fn synced(
    edit: &RefCell<TextEdit>,
    value: Signal<String>,
    caret: Signal<usize>,
) -> RefMut<'_, TextEdit> {
    let mut edit = edit.borrow_mut();
    edit.sync_external(value.peek());
    if edit.caret() != caret.peek() {
        edit.place_caret(caret.peek());
    }
    edit
}
