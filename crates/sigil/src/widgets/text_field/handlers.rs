use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;

use super::super::text_edit::{
    KeyProfile, TextEditRuntime, attach_a11y_edit, attach_ime_handlers, clipboard_keys, edit_keys,
    history_keys, move_caret, submit_text, word_keys,
};

#[derive(Clone)]
pub(super) struct TextFieldRuntime {
    pub(super) input: TextEditRuntime,
    pub(super) password: bool,
    pub(super) reveal: Signal<bool>,
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let input = &runtime.input;
    let edge = input.clone();
    let profile = KeyProfile {
        edge: Rc::new(move |event, end, extend| {
            move_caret(event, &edge, |edit| {
                let to = if end { edit.len() } else { 0 };
                edit.jump_to(to, extend);
            });
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

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
