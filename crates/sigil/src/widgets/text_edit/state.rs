use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use crate::reactive::{Signal, batch, signal};

use super::super::TextEdit;

#[derive(Clone)]
pub struct TextEditState {
    pub(in crate::widgets::text_edit) value: Signal<String>,
    pub(in crate::widgets::text_edit) caret: Signal<usize>,
    pub(super) selection: Signal<Option<usize>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
}

impl TextEditState {
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

    pub fn select(&self, byte: usize, extend: bool) {
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        if edit.caret() != self.caret.peek() {
            edit.place_caret(self.caret.peek());
        }
        if extend {
            edit.extend_to(byte);
        } else {
            edit.place_caret(byte);
        }
        batch(|| {
            self.caret.set(edit.caret());
            self.selection.set(edit.selection());
        });
    }

    pub fn insert(&self, text: &str) {
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        if edit.caret() != self.caret.peek() {
            edit.place_caret(self.caret.peek());
        }
        edit.insert_str(text);
        batch(|| {
            super::write_value(self.value, edit.as_str());
            self.caret.set(edit.caret());
            self.selection.set(edit.selection());
        });
    }

    pub fn replace(&self, text: &str) {
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        if edit.caret() != self.caret.peek() {
            edit.place_caret(self.caret.peek());
        }
        edit.replace(text);
        batch(|| {
            super::write_value(self.value, edit.as_str());
            self.caret.set(edit.caret());
            self.selection.set(edit.selection());
        });
    }

    #[must_use]
    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.selection.get()?;
        let caret = self.caret.get();
        Some(anchor.min(caret)..anchor.max(caret))
    }
}
