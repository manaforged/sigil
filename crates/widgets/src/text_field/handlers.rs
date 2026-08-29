use std::cell::RefCell;
use std::rc::Rc;

use sigil_events::prelude::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use sigil_kernel::NodeBuilder;
use sigil_layout::with_text_geometry;
use sigil_reactive::{Signal, batch, signal};
use sigil_style::TextStyle;

use super::super::clipboard::Clipboard;
use super::super::shared::disabled_now;
use super::super::text_edit::TextEdit;
use super::builder::TextChange;
use super::ime::ImeState;

#[derive(Clone)]
pub(super) struct TextFieldRuntime {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
    pub(super) scroll: Signal<f32>,
    pub(super) column: f32,
    pub(super) caret: Signal<usize>,
    pub(super) selection: Signal<Option<usize>>,
    pub(super) text_style: Signal<TextStyle>,
    pub(super) ime: ImeState,
    pub(super) clip_fail: Signal<bool>,
}

impl TextFieldRuntime {
    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    pub(super) fn new(
        value: Signal<String>,
        disabled: Option<Signal<bool>>,
        change: TextChange,
        submit: TextChange,
        clipboard: Rc<dyn Clipboard>,
        caret_visible: Signal<bool>,
        scroll: Signal<f32>,
        column: f32,
    ) -> Self {
        let edit = TextEdit::new(value.peek());
        let caret = signal(edit.caret());
        let selection = signal(edit.selection());
        let text_style = signal(TextStyle::default());
        Self {
            value,
            disabled,
            edit: Rc::new(RefCell::new(edit)),
            change,
            submit,
            clipboard,
            caret_visible,
            scroll,
            column,
            caret,
            selection,
            text_style,
            ime: ImeState::new(),
            clip_fail: signal(false),
        }
    }

    pub(super) fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }
}

pub(super) fn noop_change() -> TextChange {
    Rc::new(|_| {})
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let node = edit_keys(node, runtime.clone());
    let node = shift_keys(node, runtime.clone());
    let node = clipboard_keys(node, runtime.clone());
    history_keys(node, runtime)
}

fn edit_keys(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let any_key = runtime.clone();
    let backspace = runtime.clone();
    let delete = runtime.clone();
    let left = runtime.clone();
    let right = runtime.clone();
    let home = runtime.clone();
    let end = runtime.clone();
    let submit = runtime;
    node.key_any(move |event| insert_text(event, &any_key))
        .key(Key::Backspace, move |event| {
            edit_value(event, &backspace, TextEdit::backspace);
        })
        .key(Key::Delete, move |event| {
            edit_value(event, &delete, TextEdit::delete_forward);
        })
        .key(Key::ArrowLeft, move |event| {
            move_caret(event, &left, TextEdit::move_left);
        })
        .key(Key::ArrowRight, move |event| {
            move_caret(event, &right, TextEdit::move_right);
        })
        .key(Key::Home, move |event| {
            move_caret(event, &home, TextEdit::move_home);
        })
        .key(Key::End, move |event| {
            move_caret(event, &end, TextEdit::move_end);
        })
        .key(Key::Enter, move |event| submit_text(event, &submit))
}

fn shift_keys(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let shift_left = runtime.clone();
    let shift_right = runtime.clone();
    let shift_home = runtime.clone();
    let shift_end = runtime;
    node.key(
        Shortcut::new(Key::ArrowLeft, ModifiersState::SHIFT),
        move |event| {
            move_caret(event, &shift_left, TextEdit::shift_left);
        },
    )
    .key(
        Shortcut::new(Key::ArrowRight, ModifiersState::SHIFT),
        move |event| {
            move_caret(event, &shift_right, TextEdit::shift_right);
        },
    )
    .key(
        Shortcut::new(Key::Home, ModifiersState::SHIFT),
        move |event| {
            move_caret(event, &shift_home, TextEdit::shift_home);
        },
    )
    .key(
        Shortcut::new(Key::End, ModifiersState::SHIFT),
        move |event| {
            move_caret(event, &shift_end, TextEdit::shift_end);
        },
    )
}

fn clipboard_keys(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let copy = runtime.clone();
    let cut = runtime.clone();
    let paste = runtime.clone();
    let select_all = runtime;
    node.key(Shortcut::ctrl(LogicalKey::character("c")), move |event| {
        clipboard_copy(event, &copy);
    })
    .key(Shortcut::ctrl(LogicalKey::character("x")), move |event| {
        clipboard_cut(event, &cut);
    })
    .key(Shortcut::ctrl(LogicalKey::character("v")), move |event| {
        clipboard_paste(event, &paste);
    })
    .key(Shortcut::ctrl(LogicalKey::character("a")), move |event| {
        select_all_text(event, &select_all);
    })
}

fn history_keys(node: NodeBuilder, runtime: TextFieldRuntime) -> NodeBuilder {
    let redo_shift = runtime.clone();
    let redo_ctrl_y = runtime.clone();
    let undo_rt = runtime;
    node.key(
        Shortcut::new(
            LogicalKey::character("z"),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ),
        move |event| {
            history_redo(event, &redo_shift);
        },
    )
    .key(Shortcut::ctrl(LogicalKey::character("z")), move |event| {
        history_undo(event, &undo_rt);
    })
    .key(Shortcut::ctrl(LogicalKey::character("y")), move |event| {
        history_redo(event, &redo_ctrl_y);
    })
}

fn insert_text(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    if event.modifiers.contains(ModifiersState::CONTROL)
        || event.modifiers.contains(ModifiersState::META)
    {
        return;
    }
    let Some(text) = event.text.as_deref().filter(|text| insertable(text)) else {
        return;
    };
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str(text);
    publish(runtime, &edit);
    event.stop_propagation();
}

fn edit_value(event: &mut KeyEvent, runtime: &TextFieldRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    let before = (state.len(), state.is_empty());
    edit(&mut state);
    batch(|| {
        if (state.len(), state.is_empty()) != before {
            write_value(runtime.value, state.as_str());
            (runtime.change)(state.as_str());
        }
        commit(runtime, &state);
    });
}

fn move_caret(event: &mut KeyEvent, runtime: &TextFieldRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit.borrow_mut();
        edit(&mut state);
        debug_assert!(state.caret() <= state.len());
        commit(runtime, &state);
    }
}

fn clipboard_copy(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let state = runtime.edit.borrow();
    if let Some(text) = state.selected_text() {
        runtime
            .clip_fail
            .set(crate::clipboard::write(&*runtime.clipboard, text).is_err());
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit.borrow().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        if crate::clipboard::write(&*runtime.clipboard, &text).is_ok() {
            runtime.clip_fail.set(false);
            let mut state = runtime.edit.borrow_mut();
            state.backspace();
            publish(runtime, &state);
        } else {
            runtime.clip_fail.set(true);
        }
    }
}

fn clipboard_paste(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let Some(text) = runtime.clipboard.get() else {
        return;
    };
    let mut state = runtime.edit.borrow_mut();
    for ch in text.chars().filter(|c| *c != '\n' && *c != '\r') {
        let buf = {
            let mut b = [0u8; 4];
            let s = ch.encode_utf8(&mut b);
            s.to_owned()
        };
        state.insert_str(&buf);
    }
    publish(runtime, &state);
}

fn select_all_text(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit.borrow_mut();
        state.select_all();
        commit(runtime, &state);
    }
}

fn history_undo(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    if state.undo() {
        publish(runtime, &state);
    }
}

fn history_redo(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    if state.redo() {
        publish(runtime, &state);
    }
}

fn submit_text(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let value = runtime.value.peek();
    (runtime.submit)(value.as_str());
}

pub(super) fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}

pub(super) fn publish(runtime: &TextFieldRuntime, state: &TextEdit) {
    batch(|| {
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
        commit(runtime, state);
    });
}

pub(super) fn commit(runtime: &TextFieldRuntime, state: &TextEdit) {
    update_scroll(state, runtime);
    runtime.caret_visible.set(true);
    if runtime.caret.peek() != state.caret() {
        runtime.caret.set(state.caret());
    }
    if runtime.selection.peek() != state.selection() {
        runtime.selection.set(state.selection());
    }
}

pub(super) fn update_scroll(edit: &TextEdit, runtime: &TextFieldRuntime) {
    let style = runtime.text_style.peek();
    let Some(caret_x) =
        with_text_geometry(|engine| engine.caret(edit.as_str(), &style, edit.caret()).x)
    else {
        return;
    };
    let mut offset = runtime.scroll.peek();
    if caret_x < offset {
        offset = caret_x;
    } else if caret_x > offset + runtime.column {
        offset = caret_x - runtime.column;
    }
    let offset = offset.max(0.0);
    if (offset - runtime.scroll.peek()).abs() > f32::EPSILON {
        runtime.scroll.set(offset);
    }
}

pub(super) fn insertable(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(char::is_control)
}
