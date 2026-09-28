use std::cell::RefMut;
use std::rc::Rc;

use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};

use super::super::text_edit::{
    KeyProfile, TextEdit, TextEditRuntime, TextEditState, attach_a11y_edit, attach_ime_handlers,
    clipboard_keys, edit_keys, history_keys, move_caret, submit_text, word_keys,
};
use super::builder::TextChange;
use crate::clipboard::Clipboard;

#[derive(Clone)]
pub(super) struct TextFieldRuntime {
    pub(super) input: TextEditRuntime,
    pub(super) password: bool,
    pub(super) reveal: Signal<bool>,
}

pub(super) struct FieldConfig {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
    pub(super) scroll: Signal<f32>,
    pub(super) column: f32,
    pub(super) password: bool,
    pub(super) reveal: Signal<bool>,
}

impl TextFieldRuntime {
    pub(super) fn new(config: FieldConfig) -> Self {
        let caret = signal(config.value.peek().len());
        let input = TextEditRuntime::new(
            TextEditState::new(config.value, caret),
            config.disabled,
            config.change,
            config.submit,
            config.clipboard,
            config.caret_visible,
            (0.0, 0.0),
        )
        .single_line(config.column, config.scroll);
        Self {
            input,
            password: config.password,
            reveal: config.reveal,
        }
    }

    pub(super) fn edit_mut(&self) -> RefMut<'_, TextEdit> {
        self.input.edit_mut()
    }

    pub(super) fn disabled(&self) -> bool {
        self.input.disabled()
    }
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let input = &runtime.input;
    let edge = input.clone();
    let profile = KeyProfile {
        edge: Rc::new(move |event, end, extend| {
            move_caret(event, &edge, |edit| edge_move(edit, end, extend));
        }),
        line: None,
    };
    let submit = input.clone();
    let node =
        edit_keys(node, input, &profile).key(Key::Enter, move |event| submit_text(event, &submit));
    let node = clipboard_keys(node, input);
    let node = word_keys(node, input);
    let node = history_keys(node, input.clone());
    let node = attach_a11y_edit(node, input.clone());
    attach_ime_handlers(node, input)
}

fn edge_move(edit: &mut TextEdit, end: bool, extend: bool) {
    match (end, extend) {
        (false, false) => edit.move_home(),
        (true, false) => edit.move_end(),
        (false, true) => edit.shift_home(),
        (true, true) => edit.shift_end(),
    }
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
