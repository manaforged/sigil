use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use crate::events::prelude::{Key, KeyEvent, ModifiersState, NodeEventsExt, Shortcut};
use crate::hal::TextAffinity;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, batch, signal};
use crate::style::TextStyle;

use super::super::shared::disabled_now;
use super::TextEdit;
use super::builder::TextChange;
use super::ime::ImeState;
use crate::clipboard::Clipboard;

#[path = "commands.rs"]
mod commands;
#[path = "navigation.rs"]
mod navigation;
#[path = "state.rs"]
mod state;
pub(in crate::widgets) use commands::{
    attach_a11y_edit, clipboard_keys, history_keys, submit_text, word_keys,
};
pub(super) use navigation::geometry_width;
use navigation::{move_horizontal, move_vertical, move_visual_edge, update_scroll};
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
        commit(self, &edit);
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

pub(in crate::widgets) fn commit(runtime: &TextEditRuntime, state: &TextEdit) {
    if let Some(column) = runtime.column {
        update_scroll(runtime, state, column);
    }
    runtime.preferred_x.set(None);
    runtime.caret_affinity.set(TextAffinity::Downstream);
    runtime.caret_visible.set(true);
    if runtime.caret.peek() != state.caret() {
        runtime.caret.set(state.caret());
    }
    if runtime.selection.peek() != state.selection() {
        runtime.selection.set(state.selection());
    }
}

pub(in crate::widgets) fn noop_change() -> TextChange {
    Rc::new(|_| {})
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let node = edit_keys(node, runtime.clone());
    let node = shift_keys(node, runtime.clone());
    let node = commands::clipboard_keys(node, &runtime);
    let node = word_keys(node, &runtime);
    let node = commands::history_keys(node, runtime.clone());
    commands::attach_a11y_edit(node, runtime)
}

fn edit_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let any_key = runtime.clone();
    let backspace = runtime.clone();
    let delete = runtime.clone();
    let left = runtime.clone();
    let right = runtime.clone();
    let up = runtime.clone();
    let down = runtime.clone();
    let home = runtime.clone();
    let end = runtime.clone();
    let page_up = runtime.clone();
    let page_down = runtime.clone();
    let enter = runtime.clone();
    let submit = runtime;
    node.key_any(move |event| insert_text(event, &any_key))
        .key(Key::Backspace, move |event| {
            edit_value(event, &backspace, TextEdit::backspace);
        })
        .key(Key::Delete, move |event| {
            edit_value(event, &delete, TextEdit::delete_forward);
        })
        .key(Key::ArrowLeft, move |event| {
            move_horizontal(event, &left, false, false);
        })
        .key(Key::ArrowRight, move |event| {
            move_horizontal(event, &right, true, false);
        })
        .key(Key::ArrowUp, move |event| {
            move_vertical(event, &up, -1, false);
        })
        .key(Key::ArrowDown, move |event| {
            move_vertical(event, &down, 1, false);
        })
        .key(Key::Home, move |event| {
            move_visual_edge(event, &home, false, false);
        })
        .key(Key::End, move |event| {
            move_visual_edge(event, &end, true, false);
        })
        .key(Key::PageUp, move |event| {
            move_vertical(event, &page_up, -10, false);
        })
        .key(Key::PageDown, move |event| {
            move_vertical(event, &page_down, 10, false);
        })
        .key(Key::Enter, move |event| insert_newline(event, &enter))
        .key(
            Shortcut::new(Key::Enter, ModifiersState::CONTROL),
            move |event| commands::submit_text(event, &submit),
        )
}

fn shift_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let left = runtime.clone();
    let right = runtime.clone();
    let home = runtime.clone();
    let end = runtime.clone();
    let up = runtime.clone();
    let down = runtime.clone();
    let page_up = runtime.clone();
    let page_down = runtime;
    node.key(
        Shortcut::new(Key::ArrowLeft, ModifiersState::SHIFT),
        move |event| move_horizontal(event, &left, false, true),
    )
    .key(
        Shortcut::new(Key::ArrowRight, ModifiersState::SHIFT),
        move |event| move_horizontal(event, &right, true, true),
    )
    .key(
        Shortcut::new(Key::ArrowUp, ModifiersState::SHIFT),
        move |event| move_vertical(event, &up, -1, true),
    )
    .key(
        Shortcut::new(Key::ArrowDown, ModifiersState::SHIFT),
        move |event| move_vertical(event, &down, 1, true),
    )
    .key(
        Shortcut::new(Key::Home, ModifiersState::SHIFT),
        move |event| {
            move_visual_edge(event, &home, false, true);
        },
    )
    .key(
        Shortcut::new(Key::End, ModifiersState::SHIFT),
        move |event| {
            move_visual_edge(event, &end, true, true);
        },
    )
    .key(
        Shortcut::new(Key::PageUp, ModifiersState::SHIFT),
        move |event| move_vertical(event, &page_up, -10, true),
    )
    .key(
        Shortcut::new(Key::PageDown, ModifiersState::SHIFT),
        move |event| move_vertical(event, &page_down, 10, true),
    )
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
        .and_then(|text| typed_text(runtime, text))
    else {
        return;
    };
    let mut edit = runtime.edit_mut();
    edit.insert_str(&text);
    sync_value(runtime, &edit);
    event.stop_propagation();
}

fn insert_newline(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    if event.modifiers.contains(ModifiersState::CONTROL) {
        return;
    }
    let mut edit = runtime.edit_mut();
    edit.insert_str("\n");
    sync_value(runtime, &edit);
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
        commit(runtime, &state);
    } else {
        sync_value(runtime, &state);
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
        commit(runtime, &state);
    }
}

pub(in crate::widgets) fn sync_value(runtime: &TextEditRuntime, state: &TextEdit) {
    batch(|| {
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
        commit(runtime, state);
    });
}

pub(super) fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}

fn typed_text(runtime: &TextEditRuntime, text: &str) -> Option<String> {
    if runtime.column.is_some() {
        return (!text.is_empty() && !text.chars().any(char::is_control)).then(|| text.to_owned());
    }
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    (text != "\n" && is_printable(&text)).then_some(text)
}

fn is_printable(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| !ch.is_control() || matches!(ch, '\t' | '\n'))
}
