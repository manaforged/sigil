use std::cell::RefCell;
use std::rc::Rc;

use events::prelude::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use kernel::NodeBuilder;
use reactive::Signal;

use super::super::shared::disabled_now;
use super::super::text_field::Clipboard;
use super::TextEdit;
use super::builder::TextChange;

pub(super) struct NoopClipboard;

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) {}
    fn get(&self) -> Option<String> {
        None
    }
}

#[derive(Clone)]
pub(super) struct TextEditRuntime {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
}

impl TextEditRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        value: Signal<String>,
        disabled: Option<Signal<bool>>,
        change: TextChange,
        submit: TextChange,
        clipboard: Rc<dyn Clipboard>,
        caret_visible: Signal<bool>,
    ) -> Self {
        Self {
            value,
            disabled,
            edit: Rc::new(RefCell::new(TextEdit::new(value.peek()))),
            change,
            submit,
            clipboard,
            caret_visible,
        }
    }

    pub(super) fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }
}

pub(super) fn noop_change() -> TextChange {
    Rc::new(|_| {})
}

pub(super) fn attach_key_handlers(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let node = edit_keys(node, runtime.clone());
    let node = shift_keys(node, runtime.clone());
    let node = clipboard_keys(node, runtime.clone());
    history_keys(node, runtime)
}

fn edit_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let any_key = runtime.clone();
    let backspace = runtime.clone();
    let delete = runtime.clone();
    let left = runtime.clone();
    let right = runtime.clone();
    let home = runtime.clone();
    let end = runtime.clone();
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
        .key(Key::Enter, move |event| insert_newline(event, &enter))
        .key(
            Shortcut::new(Key::Enter, ModifiersState::CONTROL),
            move |event| submit_text(event, &submit),
        )
}

fn shift_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
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

fn clipboard_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
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

fn history_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
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

fn insert_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    if event.modifiers.contains(ModifiersState::CONTROL)
        || event.modifiers.contains(ModifiersState::META)
    {
        return;
    }
    let Some(text) = event.text.as_deref().filter(|text| is_printable(text)) else {
        return;
    };
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str(text);
    sync_value(runtime, &edit);
    event.stop_propagation();
}

fn insert_newline(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    // Reject Ctrl+Enter here so the shortcut handler can fire instead.
    if event.modifiers.contains(ModifiersState::CONTROL) {
        return;
    }
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str("\n");
    sync_value(runtime, &edit);
}

fn edit_value(event: &mut KeyEvent, runtime: &TextEditRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    let before = (state.len(), state.is_empty());
    edit(&mut state);
    if (state.len(), state.is_empty()) == before {
        runtime.caret_visible.set(true);
    } else {
        sync_value(runtime, &state);
    }
}

fn move_caret(event: &mut KeyEvent, runtime: &TextEditRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit.borrow_mut();
        edit(&mut state);
        debug_assert!(state.caret() <= state.len());
        runtime.caret_visible.set(true);
    }
}

fn clipboard_copy(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let state = runtime.edit.borrow();
    if let Some(text) = state.selected_text() {
        runtime.clipboard.set(text);
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit.borrow().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        runtime.clipboard.set(&text);
        let mut state = runtime.edit.borrow_mut();
        state.backspace();
        sync_value(runtime, &state);
    }
}

fn clipboard_paste(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let Some(text) = runtime.clipboard.get() else {
        return;
    };
    let mut state = runtime.edit.borrow_mut();
    // text_edit preserves newlines from clipboard; only normalise \r\n.
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    state.insert_str(&normalised);
    sync_value(runtime, &state);
}

fn select_all_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if !runtime.disabled() {
        runtime.edit.borrow_mut().select_all();
        runtime.caret_visible.set(true);
    }
}

fn history_undo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    if state.undo() {
        sync_value(runtime, &state);
    }
}

fn history_redo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    if state.redo() {
        sync_value(runtime, &state);
    }
}

fn submit_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let value = runtime.value.peek();
    (runtime.submit)(value.as_str());
}

fn sync_value(runtime: &TextEditRuntime, state: &TextEdit) {
    write_value(runtime.value, state.as_str());
    (runtime.change)(state.as_str());
    runtime.caret_visible.set(true);
}

fn write_value(value: Signal<String>, next: &str) {
    value.update(|current| {
        current.clear();
        current.push_str(next);
    });
}

/// Accept any text payload whose chars are not control codes (newlines
/// arrive on the `Enter` key, not the text payload).
fn is_printable(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| !ch.is_control() || ch == '\t')
}
