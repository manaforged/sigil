use std::cell::{RefCell, RefMut};
use std::ops::Range;
use std::rc::Rc;

use crate::reactive::{Signal, batch, signal};

use super::super::TextEdit;

/// Caret and undo state of a text editor.
#[derive(Clone)]
pub struct TextEditState {
    pub(in crate::widgets::text_edit) value: Signal<String>,
    pub(in crate::widgets::text_edit) caret: Signal<usize>,
    pub(super) selection: Signal<Option<usize>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
}

impl std::fmt::Debug for TextEditState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEditState")
            .field("value", &self.value)
            .field("caret", &self.caret)
            .field("selection", &self.selection)
            .finish_non_exhaustive()
    }
}

impl TextEditState {
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
        }
    }

    /// Moves the caret to `byte` and extends the selection when `extend` is true.
    pub fn select(&self, byte: usize, extend: bool) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        if extend {
            edit.extend_to(byte);
        } else {
            edit.place_caret(byte);
        }
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

    fn publish(&self, edit: &TextEdit, value: bool) {
        batch(|| {
            if value {
                super::write_value(self.value, edit.as_str());
            }
            self.caret.set(edit.caret());
            self.selection.set(edit.selection());
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
