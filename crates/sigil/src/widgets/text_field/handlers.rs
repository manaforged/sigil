use std::cell::RefCell;
use std::rc::Rc;

use events::prelude::{Key, KeyEvent, LogicalKey, ModifiersState, NodeEventsExt, Shortcut};
use kernel::NodeBuilder;
use layout::with_text_geometry;
use reactive::{Signal, signal};
use style::TextStyle;

use super::super::shared::disabled_now;
use super::super::text_edit::TextEdit;
use super::builder::{Clipboard, TextChange};
use super::ime::ImeState;

pub(super) struct NoopClipboard;

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) {}
    fn get(&self) -> Option<String> {
        None
    }
}

#[derive(Clone)]
pub(super) struct TextFieldRuntime {
    pub(super) value: Signal<String>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) edit: Rc<RefCell<TextEdit>>,
    pub(super) change: TextChange,
    pub(super) submit: TextChange,
    pub(super) clipboard: Rc<dyn Clipboard>,
    pub(super) caret_visible: Signal<bool>,
    /// Horizontal scroll of the text track in logical px (>= 0). The render
    /// layer translates the glyph/caret/selection track left by this much so
    /// the caret stays inside the clipped viewport. Pixel-, not character-,
    /// based so it stays correct at any field width and font.
    pub(super) scroll: Signal<f32>,
    /// Visible inner column width in logical px — the viewport the caret must
    /// stay within. Fed into the scroll math so a wider field scrolls later.
    pub(super) column: f32,
    /// Caret byte offset mirrored from the buffer for the render overlay.
    pub(super) caret: Signal<usize>,
    /// Selection anchor mirrored from the buffer (`None` = collapsed caret).
    pub(super) selection: Signal<Option<usize>>,
    /// Widget-local IME composition state (preedit text + caret anchor).
    /// See `super::ime` — DR-051's "widget-local signal" default.
    pub(super) ime: ImeState,
}

impl TextFieldRuntime {
    #[allow(clippy::too_many_arguments)]
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
            ime: ImeState::new(),
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
    let Some(text) = event
        .text
        .as_deref()
        .filter(|text| is_single_line_scalar(text))
    else {
        return;
    };
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str(text);
    write_value(runtime.value, edit.as_str());
    (runtime.change)(edit.as_str());
    commit(runtime, &edit);
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
    if (state.len(), state.is_empty()) != before {
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
    }
    commit(runtime, &state);
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
        runtime.clipboard.set(text);
    }
}

fn clipboard_cut(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let selected = runtime.edit.borrow().selected_text().map(ToOwned::to_owned);
    if let Some(text) = selected {
        runtime.clipboard.set(&text);
        let mut state = runtime.edit.borrow_mut();
        state.backspace();
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
        commit(runtime, &state);
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
    write_value(runtime.value, state.as_str());
    (runtime.change)(state.as_str());
    commit(runtime, &state);
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
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
        commit(runtime, &state);
    }
}

fn history_redo(event: &mut KeyEvent, runtime: &TextFieldRuntime) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit.borrow_mut();
    if state.redo() {
        write_value(runtime.value, state.as_str());
        (runtime.change)(state.as_str());
        commit(runtime, &state);
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

/// Publish post-mutation state: keep the caret visible, scroll it into view,
/// and mirror the buffer's caret + selection into the render signals that
/// drive the caret/selection overlay. Called after every buffer mutation
/// (keyboard or pointer) so the visible caret tracks the model.
pub(super) fn commit(runtime: &TextFieldRuntime, state: &TextEdit) {
    update_scroll(state, runtime.scroll, runtime.column);
    runtime.caret_visible.set(true);
    if runtime.caret.peek() != state.caret() {
        runtime.caret.set(state.caret());
    }
    if runtime.selection.peek() != state.selection() {
        runtime.selection.set(state.selection());
    }
}

/// Slide the horizontal scroll so the caret stays inside the `column`-wide
/// viewport. Pixel-based: the caret's x comes from the real text engine
/// (`with_text_geometry`), so this is correct at any width and font — unlike
/// the old fixed "≈19 chars" guess that mis-scrolled wider fields like the
/// composer. With no geometry provider installed (headless layout tests) the
/// caret x is unknown, so scroll is left untouched (stays 0).
pub(super) fn update_scroll(edit: &TextEdit, scroll: Signal<f32>, column: f32) {
    let Some(caret_x) = with_text_geometry(|engine| {
        engine
            .caret(edit.as_str(), &TextStyle::default(), edit.caret())
            .x
    }) else {
        return;
    };
    let mut offset = scroll.peek();
    if caret_x < offset {
        // Caret moved left of the window — pin it to the left edge.
        offset = caret_x;
    } else if caret_x > offset + column {
        // Caret moved past the right edge — pin it to the right edge.
        offset = caret_x - column;
    }
    let offset = offset.max(0.0);
    if (offset - scroll.peek()).abs() > f32::EPSILON {
        scroll.set(offset);
    }
}

pub(super) fn is_single_line_scalar(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    chars.next().is_none() && ch != '\n' && ch != '\r'
}
