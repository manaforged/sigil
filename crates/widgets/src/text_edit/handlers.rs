use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use sigil_events::prelude::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, batch, signal};

use super::super::clipboard::Clipboard;
use super::super::shared::disabled_now;
use super::TextEdit;
use super::builder::TextChange;
use super::ime::ImeState;

#[derive(Clone)]
pub(super) struct TextEditRuntime {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
    /// Caret byte offset mirrored from the buffer — used to splice the
    /// IME preedit span into the displayed text and to compute the
    /// published IME caret anchor. `text_edit` has no visible caret/
    /// selection overlay yet (unlike `text_field`); this mirror exists
    /// solely for IME, not general caret rendering.
    pub(super) caret: Signal<usize>,
    /// Selection anchor mirrored from the buffer for the selection
    /// overlay. `None` means caret-only (no selection).
    pub(super) selection: Signal<Option<usize>>,
    /// Widget-local IME composition state (preedit text + caret
    /// anchor). See `super::ime`.
    pub(super) ime: ImeState,
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
        let edit = TextEdit::new(value.peek());
        let caret = signal(edit.caret());
        let selection = signal(edit.selection());
        Self {
            value,
            disabled,
            edit: Rc::new(RefCell::new(edit)),
            change,
            submit,
            clipboard,
            caret_visible,
            caret,
            selection,
            ime: ImeState::new(),
        }
    }

    pub(super) fn disabled(&self) -> bool {
        disabled_now(self.disabled)
    }

    /// Borrow the internal edit buffer for mutation, first
    /// resynchronizing it against the bound `value` signal if the app
    /// wrote to `value` directly since the last widget-originated
    /// mutation (external doc replace, host-driven undo, etc.) —
    /// otherwise a stale internal buffer would silently clobber that
    /// external write on the next keystroke or IME commit. The
    /// equality check inside `sync_external` makes this a no-op on
    /// our own writes (a widget-originated mutation always leaves
    /// `edit.as_str() == value.peek()` immediately after `commit`),
    /// so no separate origin-tracking flag is needed.
    pub(super) fn edit_mut(&self) -> RefMut<'_, TextEdit> {
        let mut edit = self.edit.borrow_mut();
        edit.sync_external(self.value.peek());
        edit
    }
}

/// Publish post-mutation state: keep the caret visible and mirror the
/// buffer's caret into the render/IME-anchor signal. Mirrors
/// `text_field::handlers::commit`, minus selection/scroll — `text_edit`
/// has no selection overlay or scroll signal to keep in sync (pre-
/// existing gap; this dispatch only wires IME, not that).
pub(super) fn commit(runtime: &TextEditRuntime, state: &TextEdit) {
    runtime.caret_visible.set(true);
    if runtime.caret.peek() != state.caret() {
        runtime.caret.set(state.caret());
    }
    if runtime.selection.peek() != state.selection() {
        runtime.selection.set(state.selection());
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
            move_caret(event, &left, TextEdit::move_left);
        })
        .key(Key::ArrowRight, move |event| {
            move_caret(event, &right, TextEdit::move_right);
        })
        .key(Key::ArrowUp, move |event| {
            move_caret(event, &up, TextEdit::move_up);
        })
        .key(Key::ArrowDown, move |event| {
            move_caret(event, &down, TextEdit::move_down);
        })
        .key(Key::Home, move |event| {
            move_caret(event, &home, TextEdit::move_home);
        })
        .key(Key::End, move |event| {
            move_caret(event, &end, TextEdit::move_end);
        })
        .key(Key::PageUp, move |event| {
            move_caret(event, &page_up, TextEdit::move_page_up);
        })
        .key(Key::PageDown, move |event| {
            move_caret(event, &page_down, TextEdit::move_page_down);
        })
        .key(Key::Enter, move |event| insert_newline(event, &enter))
        .key(
            Shortcut::new(Key::Enter, ModifiersState::CONTROL),
            move |event| submit_text(event, &submit),
        )
}

fn shift_keys(node: NodeBuilder, runtime: TextEditRuntime) -> NodeBuilder {
    let node = shift_key(node, Key::ArrowLeft, runtime.clone(), TextEdit::shift_left);
    let node = shift_key(
        node,
        Key::ArrowRight,
        runtime.clone(),
        TextEdit::shift_right,
    );
    let node = shift_key(node, Key::ArrowUp, runtime.clone(), TextEdit::shift_up);
    let node = shift_key(node, Key::ArrowDown, runtime.clone(), TextEdit::shift_down);
    let node = shift_key(node, Key::Home, runtime.clone(), TextEdit::shift_home);
    let node = shift_key(node, Key::End, runtime.clone(), TextEdit::shift_end);
    let node = shift_key(node, Key::PageUp, runtime.clone(), TextEdit::shift_page_up);
    shift_key(node, Key::PageDown, runtime, TextEdit::shift_page_down)
}

fn shift_key(
    node: NodeBuilder,
    key: Key,
    runtime: TextEditRuntime,
    edit: fn(&mut TextEdit),
) -> NodeBuilder {
    node.key(Shortcut::new(key, ModifiersState::SHIFT), move |event| {
        move_caret(event, &runtime, edit);
    })
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
    let mut edit = runtime.edit_mut();
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
    let mut edit = runtime.edit_mut();
    edit.insert_str("\n");
    sync_value(runtime, &edit);
}

fn edit_value(event: &mut KeyEvent, runtime: &TextEditRuntime, edit: impl FnOnce(&mut TextEdit)) {
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

fn move_caret(event: &mut KeyEvent, runtime: &TextEditRuntime, edit: impl FnOnce(&mut TextEdit)) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit_mut();
        edit(&mut state);
        debug_assert!(state.caret() <= state.len());
        commit(runtime, &state);
    }
}

fn clipboard_copy(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let state = runtime.edit.borrow();
    if let Some(text) = state.selected_text() {
        drop(crate::clipboard::write(&*runtime.clipboard, text));
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit.borrow().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        if crate::clipboard::write(&*runtime.clipboard, &text).is_err() {
            return;
        }
        let mut state = runtime.edit_mut();
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
    let mut state = runtime.edit_mut();
    // text_edit preserves newlines from clipboard; only normalise \r\n.
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n");
    state.insert_str(&normalised);
    sync_value(runtime, &state);
}

fn select_all_text(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if !runtime.disabled() {
        let mut state = runtime.edit_mut();
        state.select_all();
        commit(runtime, &state);
    }
}

fn history_undo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    if state.undo() {
        sync_value(runtime, &state);
    }
}

fn history_redo(event: &mut KeyEvent, runtime: &TextEditRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
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

/// Publish a mutated buffer's `value` + `caret` as one atomic reactive
/// unit. Without `batch`, `write_value`'s `Signal::update` would run
/// every dependent effect synchronously (including `text_edit`'s
/// preedit-splice `derived_eq`) BEFORE `commit` gets a chance to
/// publish the matching new caret — a reader observing `value` after
/// the shrink but `caret` before it can slice a preedit splice
/// mid-codepoint. `batch` coalesces both writes into a single drain
/// after this closure returns, so every dependent effect always sees
/// the post-mutation `(value, caret)` pair together, never the
/// half-updated one (the reviewer adversarial finding [W5-IME-TE] HIGH-1).
fn sync_value(runtime: &TextEditRuntime, state: &TextEdit) {
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

/// Accept any text payload whose chars are not control codes (newlines
/// arrive on the `Enter` key, not the text payload).
fn is_printable(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| !ch.is_control() || ch == '\t')
}
