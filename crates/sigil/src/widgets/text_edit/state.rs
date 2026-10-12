use std::cell::{RefCell, RefMut};
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
        let mut edit = synced(&self.edit, self.value, self.caret);
        if extend {
            edit.extend_to(byte);
        } else {
            edit.place_caret(byte);
        }
        self.publish(&edit, false);
    }

    pub fn insert(&self, text: &str) {
        let mut edit = synced(&self.edit, self.value, self.caret);
        edit.insert_str(text);
        self.publish(&edit, true);
    }

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
