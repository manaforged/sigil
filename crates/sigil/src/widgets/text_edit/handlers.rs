use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use crate::events::{Key, KeyEvent, Modifiers, NamedKey, Shortcut};
use crate::hal::TextAffinity;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, batch, signal};
use crate::style::TextStyle;
use crate::widgets::Widget;

pub(in crate::widgets) use super::super::shared::noop_change;
use super::super::shared::{TextChange, disabled_now};
use super::TextEdit;
use super::builder::SubmitKey;
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
    attach_a11y_edit, bind_command_keys, bind_history_keys, clipboard_keys, history_keys,
    submit_text, word_keys,
};
pub(in crate::widgets) use keymap::{KeyProfile, LineStep, edit_keys};
use navigation::{move_vertical, move_visual_edge};
pub use state::TextState;

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
    pub(in crate::widgets) clip_fail: Signal<bool>,
}

impl TextEditRuntime {
    #[expect(
        clippy::too_many_arguments,
        reason = "wires retained editor state to widget event handlers"
    )]
    pub(in crate::widgets) fn new(
        state: TextState,
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
            preferred_x: state.preferred_x,
            caret_affinity: signal(TextAffinity::Downstream),
            anchor_affinity: signal(TextAffinity::Downstream),
            clip_fail: signal(false),
        }
    }

    pub(in crate::widgets) fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }

    pub(in crate::widgets) fn edit_mut(&self) -> RefMut<'_, TextEdit> {
        state::synced(&self.edit, self.value, self.caret)
    }

    pub(in crate::widgets) fn commit(&self, state: &TextEdit) {
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
}

pub(in crate::widgets) fn typed_event_text(event: &KeyEvent, multiline: bool) -> Option<String> {
    if event
        .modifiers()
        .intersects(Modifiers::CTRL | Modifiers::META)
    {
        return None;
    }
    let raw = event.text()?;
    if multiline {
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");
        let printable = text
            .chars()
            .all(|ch| !ch.is_control() || matches!(ch, '\t' | '\n'));
        (printable && !text.is_empty() && text != "\n").then_some(text)
    } else {
        (!raw.is_empty() && raw.chars().all(|ch| !ch.is_control())).then(|| raw.to_owned())
    }
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;

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
    let node = edit_keys(node, &runtime, &profile);
    let node = enter_keys(node, &runtime, submit_key);
    let node = clipboard_keys(node, &runtime);
    let node = word_keys(node, &runtime);
    let node = history_keys(node, runtime.clone());
    attach_a11y_edit(node, runtime)
}

fn enter_keys(node: NodeBuilder, runtime: &TextEditRuntime, submit_key: SubmitKey) -> NodeBuilder {
    let submit = runtime.clone();
    let (newlines, send) = match submit_key {
        SubmitKey::CtrlEnter => (
            vec![Shortcut::plain(NamedKey::Enter)],
            Shortcut::new(NamedKey::Enter, Modifiers::CTRL),
        ),
        SubmitKey::Enter => (
            vec![
                Shortcut::new(NamedKey::Enter, Modifiers::SHIFT),
                Shortcut::new(NamedKey::Enter, Modifiers::ALT),
                Shortcut::new(Key::character("j"), Modifiers::CTRL),
            ],
            Shortcut::plain(NamedKey::Enter),
        ),
    };
    let node = newlines.into_iter().fold(node, |node, shortcut| {
        let enter = runtime.clone();
        node.key(shortcut, move |event| {
            edit_value(event, &enter, |edit| edit.insert_str("\n"));
        })
    });
    node.key(send, move |event| submit_text(event, &submit))
}

pub(in crate::widgets) fn insert_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    let Some(text) = typed_event_text(event, true) else {
        return;
    };
    let mut edit = runtime.edit_mut();
    edit.insert_str(&text);
    runtime.sync_value(&edit);
    event.stop_propagation();
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
    let before = state.len();
    edit(&mut state);
    if state.len() == before {
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
        let extend = event.modifiers.contains(Modifiers::SHIFT);
        let had_selection = state.selection().is_some();
        let affinity = runtime.caret_affinity.peek();
        edit(&mut state);
        if extend && !had_selection {
            runtime.anchor_affinity.set(affinity);
        }
        runtime.commit(&state);
    }
}

pub(super) fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}
