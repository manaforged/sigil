use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use crate::events::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use crate::hal::TextAffinity;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, batch, signal};
use crate::style::TextStyle;

use super::super::shared::disabled_now;
use super::TextEdit;
use super::builder::{SubmitKey, TextChange};
use super::ime::ImeState;
use crate::clipboard::Clipboard;

#[path = "commands.rs"]
mod commands;
#[path = "keymap.rs"]
mod keymap;
#[path = "navigation.rs"]
mod navigation;
#[path = "state.rs"]
mod state;
pub(in crate::widgets) use commands::{
    attach_a11y_edit, clipboard_keys, history_keys, submit_text, word_keys,
};
pub(in crate::widgets) use keymap::{KeyProfile, LineStep, edit_keys};
use navigation::{move_vertical, move_visual_edge};
pub use state::TextEditState;

#[derive(Clone)]
pub(in crate::widgets) struct TextEditRuntime {
    pub(in crate::widgets) value: Signal<String>,
    pub(in crate::widgets) disabled: Option<Signal<bool>>,
    pub(in crate::widgets) edit: Rc<RefCell<TextEdit>>,
    pub(in crate::widgets) change: TextChange,
    pub(in crate::widgets) submit: TextChange,
    pub(in crate::widgets) clipboard: Rc<dyn Clipboard>,
    pub(in crate::widgets) caret_visible: Signal<bool>,
    pub(in crate::widgets) caret: Signal<usize>,
    pub(in crate::widgets) selection: Signal<Option<usize>>,
    pub(in crate::widgets) ime: ImeState,
    pub(in crate::widgets) text_style: Signal<TextStyle>,
    pub(in crate::widgets) content_size: Signal<(f32, f32)>,
    pub(in crate::widgets) viewport_size: Signal<(f32, f32)>,
    pub(in crate::widgets) scroll_y: Signal<f32>,
    pub(in crate::widgets) content_padding: (f32, f32),
    pub(in crate::widgets) preferred_x: Rc<Cell<Option<f32>>>,
    pub(in crate::widgets) caret_affinity: Signal<TextAffinity>,
    pub(in crate::widgets) anchor_affinity: Signal<TextAffinity>,
    pub(in crate::widgets) scroll_x: Signal<f32>,
    pub(in crate::widgets) column: Option<f32>,
    pub(in crate::widgets) clip_fail: Signal<bool>,
}

impl TextEditRuntime {
    #[expect(
        clippy::too_many_arguments,
        reason = "wires retained editor state to widget event handlers"
    )]
    pub(in crate::widgets) fn new(
        state: TextEditState,
        disabled: Option<Signal<bool>>,
        change: TextChange,
        submit: TextChange,
        clipboard: Rc<dyn Clipboard>,
        caret_visible: Signal<bool>,
        content_padding: (f32, f32),
    ) -> Self {
        Self {
            value: state.value,
            disabled,
            edit: state.edit,
            change,
            submit,
            clipboard,
            caret_visible,
            caret: state.caret,
            selection: state.selection,
            ime: ImeState::new(),
            text_style: signal(TextStyle::default()),
            content_size: signal((0.0, 0.0)),
            viewport_size: signal((0.0, 0.0)),
            scroll_y: signal(0.0),
            content_padding,
            preferred_x: Rc::new(Cell::new(None)),
            caret_affinity: signal(TextAffinity::Downstream),
            anchor_affinity: signal(TextAffinity::Downstream),
            scroll_x: signal(0.0),
            column: None,
            clip_fail: signal(false),
        }
    }

    pub(in crate::widgets) const fn single_line(
        mut self,
        column: f32,
        scroll_x: Signal<f32>,
    ) -> Self {
        self.column = Some(column);
        self.scroll_x = scroll_x;
        self
    }

    pub(in crate::widgets) fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }

    pub(in crate::widgets) fn edit_mut(&self) -> RefMut<'_, TextEdit> {
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        let caret = self.caret.peek();
        if caret != edit.caret() {
            edit.place_caret(caret);
        }
        edit
    }

    pub(in crate::widgets) fn place_on_focus(&self, at: FocusCaret) {
        let mut edit = self.edit_mut();
        match at {
            FocusCaret::Keep => return,
            FocusCaret::End => {
                let end = edit.len();
                edit.place_caret(end);
            }
            FocusCaret::SelectAll => edit.select_all(),
        }
        self.commit(&edit);
    }

    pub(in crate::widgets) fn commit(&self, state: &TextEdit) {
        if let Some(column) = self.column {
            self.update_scroll(state, column);
        }
        self.preferred_x.set(None);
        self.caret_affinity.set(TextAffinity::Downstream);
        self.caret_visible.set(true);
        if self.caret.peek() != state.caret() {
            self.caret.set(state.caret());
        }
        if self.selection.peek() != state.selection() {
            self.selection.set(state.selection());
        }
    }

    pub(in crate::widgets) fn sync_value(&self, state: &TextEdit) {
        batch(|| {
            write_value(self.value, state.as_str());
            (self.change)(state.as_str());
            self.commit(state);
        });
    }

    fn typed_text(&self, text: &str) -> Option<String> {
        if self.column.is_some() {
            return (!text.is_empty() && !text.chars().any(char::is_control))
                .then(|| text.to_owned());
        }
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        (text != "\n" && is_printable(&text)).then_some(text)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusCaret {
    #[default]
    Keep,
    End,
    SelectAll,
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;

pub(in crate::widgets) fn noop_change() -> TextChange {
    Rc::new(|_| {})
}

pub(super) fn attach_key_handlers(
    node: NodeBuilder,
    runtime: TextEditRuntime,
    submit_key: SubmitKey,
) -> NodeBuilder {
    let edge = runtime.clone();
    let line = runtime.clone();
    let profile = KeyProfile {
        edge: Rc::new(move |event, end, extend| move_visual_edge(event, &edge, end, extend)),
        line: Some(Rc::new(move |event, step, extend| {
            move_vertical(event, &line, step, extend);
        })),
    };
    let node = keymap::edit_keys(node, &runtime, &profile);
    let node = enter_keys(node, &runtime, submit_key);
    let node = commands::clipboard_keys(node, &runtime);
    let node = word_keys(node, &runtime);
    let node = commands::history_keys(node, runtime.clone());
    commands::attach_a11y_edit(node, runtime)
}

fn enter_keys(node: NodeBuilder, runtime: &TextEditRuntime, submit_key: SubmitKey) -> NodeBuilder {
    let submit = runtime.clone();
    let (newlines, send) = match submit_key {
        SubmitKey::CtrlEnter => (
            vec![Shortcut::plain(Key::Enter)],
            Shortcut::new(Key::Enter, ModifiersState::CONTROL),
        ),
        SubmitKey::Enter => (
            vec![
                Shortcut::new(Key::Enter, ModifiersState::SHIFT),
                Shortcut::new(Key::Enter, ModifiersState::ALT),
                Shortcut::new(LogicalKey::character("j"), ModifiersState::CONTROL),
            ],
            Shortcut::plain(Key::Enter),
        ),
    };
    let node = newlines.into_iter().fold(node, |node, shortcut| {
        let enter = runtime.clone();
        node.key(shortcut, move |event| insert_newline(event, &enter))
    });
    node.key(send, move |event| commands::submit_text(event, &submit))
}

pub(in crate::widgets) fn insert_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    if event.modifiers.contains(ModifiersState::CONTROL)
        || event.modifiers.contains(ModifiersState::META)
    {
        return;
    }
    let Some(text) = event
        .text
        .as_deref()
        .and_then(|text| runtime.typed_text(text))
    else {
        return;
    };
    let mut edit = runtime.edit_mut();
    edit.insert_str(&text);
    runtime.sync_value(&edit);
    event.stop_propagation();
}

fn insert_newline(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut edit = runtime.edit_mut();
    edit.insert_str("\n");
    runtime.sync_value(&edit);
}

pub(in crate::widgets) fn edit_value(
    event: &mut KeyEvent,
    runtime: &TextEditRuntime,
    edit: impl FnOnce(&mut TextEdit),
) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    let before = (state.len(), state.is_empty());
    edit(&mut state);
    if (state.len(), state.is_empty()) == before {
        runtime.commit(&state);
    } else {
        runtime.sync_value(&state);
    }
}

pub(in crate::widgets) fn move_caret(
    event: &mut KeyEvent,
    runtime: &TextEditRuntime,
    edit: impl FnOnce(&mut TextEdit),
) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit_mut();
        let extend = event.modifiers.contains(ModifiersState::SHIFT);
        let had_selection = state.selection().is_some();
        let affinity = runtime.caret_affinity.peek();
        edit(&mut state);
        if extend && !had_selection {
            runtime.anchor_affinity.set(affinity);
        }
        debug_assert!(state.caret() <= state.len());
        runtime.commit(&state);
    }
}

pub(super) fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}

fn is_printable(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| !ch.is_control() || matches!(ch, '\t' | '\n'))
}
