use std::cell::RefMut;
use std::rc::Rc;

use crate::events::prelude::{Key, ModifiersState, NodeEventsExt, Shortcut};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};

use super::super::text_edit::{
    TextEdit, TextEditRuntime, TextEditState, attach_a11y_edit, attach_ime_handlers,
    clipboard_keys, edit_value, history_keys, insert_text, move_caret, submit_text, word_keys,
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
    let node = edit_keys(node, input);
    let node = shift_keys(node, input);
    let node = clipboard_keys(node, input);
    let node = word_keys(node, input);
    let node = history_keys(node, input.clone());
    let node = attach_a11y_edit(node, input.clone());
    attach_ime_handlers(node, input)
}

type Edit = fn(&mut TextEdit);

fn edit_keys(mut node: NodeBuilder, input: &TextEditRuntime) -> NodeBuilder {
    let any_key = input.clone();
    let submit = input.clone();
    node = node
        .key_any(move |event| insert_text(event, &any_key))
        .key(Key::Enter, move |event| submit_text(event, &submit));
    let edits: [(Key, Edit); 2] = [
        (Key::Backspace, TextEdit::backspace),
        (Key::Delete, TextEdit::delete_forward),
    ];
    for (key, edit) in edits {
        let input = input.clone();
        node = node.key(key, move |event| edit_value(event, &input, edit));
    }
    let moves: [(Key, Edit); 4] = [
        (Key::ArrowLeft, TextEdit::move_left),
        (Key::ArrowRight, TextEdit::move_right),
        (Key::Home, TextEdit::move_home),
        (Key::End, TextEdit::move_end),
    ];
    for (key, edit) in moves {
        let input = input.clone();
        node = node.key(key, move |event| move_caret(event, &input, edit));
    }
    node
}

fn shift_keys(mut node: NodeBuilder, input: &TextEditRuntime) -> NodeBuilder {
    let moves: [(Key, Edit); 4] = [
        (Key::ArrowLeft, TextEdit::shift_left),
        (Key::ArrowRight, TextEdit::shift_right),
        (Key::Home, TextEdit::shift_home),
        (Key::End, TextEdit::shift_end),
    ];
    for (key, edit) in moves {
        let input = input.clone();
        node = node.key(Shortcut::new(key, ModifiersState::SHIFT), move |event| {
            move_caret(event, &input, edit);
        });
    }
    node
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
