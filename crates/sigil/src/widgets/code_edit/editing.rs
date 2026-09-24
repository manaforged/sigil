use crate::clipboard::clipboard;
use crate::events::prelude::{Key, KeyEvent, ModifiersState, NodeEventsExt, Shortcut};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, batch, effect, signal};
use crate::widgets::grapheme::col_count;
use crate::widgets::text_edit::{
    TextEdit, TextEditRuntime, TextEditState, attach_a11y_edit, attach_ime_handlers,
    clipboard_keys, edit_value, history_keys, insert_text, move_caret, noop_change, word_keys,
};

use super::SelectionAnchor;
use super::a11y::{byte_at, point_at};

const PAGE_ROWS: isize = 10;

type Edit = fn(&mut TextEdit);

#[derive(Clone, Copy)]
pub(super) struct Model {
    pub(super) lines: Signal<Vec<String>>,
    pub(super) caret: Signal<(usize, usize)>,
    pub(super) selection: Option<SelectionAnchor>,
}

pub(super) fn runtime(model: Model, disabled: Option<Signal<bool>>) -> TextEditRuntime {
    let lines = model.lines.peek();
    let value = signal(lines.join("\n"));
    let caret = signal(byte_at(&lines, model.caret.peek()));
    let runtime = TextEditRuntime::new(
        TextEditState::new(value, caret),
        disabled,
        noop_change(),
        noop_change(),
        clipboard(),
        signal(true),
        (0.0, 0.0),
    );
    sync_inward(model, &runtime);
    sync_outward(model, &runtime);
    runtime
}

fn sync_inward(model: Model, runtime: &TextEditRuntime) {
    let (value, caret, selection) = (runtime.value, runtime.caret, runtime.selection);
    effect(move || {
        let lines = model.lines.get();
        let joined = lines.join("\n");
        let at = byte_at(&lines, model.caret.get());
        let anchor = model
            .selection
            .and_then(Signal::get)
            .map(|point| byte_at(&lines, point));
        batch(|| {
            if value.peek() != joined {
                value.set(joined);
            }
            if caret.peek() != at {
                caret.set(at);
            }
            if selection.peek() != anchor {
                selection.set(anchor);
            }
        });
    });
}

fn sync_outward(model: Model, runtime: &TextEditRuntime) {
    let (value, caret, selection) = (runtime.value, runtime.caret, runtime.selection);
    effect(move || {
        let text = value.get();
        let (at, anchor) = (caret.get(), selection.get());
        let current = model.lines.peek();
        let lines = if current.join("\n") == text {
            current
        } else {
            text.split('\n').map(str::to_owned).collect()
        };
        let model_anchor = model.selection.and_then(Signal::peek);
        batch(|| {
            if byte_at(&lines, model.caret.peek()) != at {
                model.caret.set(point_at(&lines, at));
            }
            if let Some(signal) = model.selection
                && model_anchor.map(|point| byte_at(&lines, point)) != anchor
            {
                signal.set(anchor.map(|byte| point_at(&lines, byte)));
            }
            if model.lines.peek() != lines {
                model.lines.set(lines);
            }
        });
    });
}

pub(super) fn attach(node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let node = edit_keys(node, runtime);
    let node = line_keys(node, runtime);
    let node = clipboard_keys(node, runtime);
    let node = word_keys(node, runtime);
    let node = history_keys(node, runtime.clone());
    let node = attach_a11y_edit(node, runtime.clone());
    attach_ime_handlers(node, runtime)
}

fn edit_keys(mut node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let any_key = runtime.clone();
    let enter = runtime.clone();
    node = node
        .key_any(move |event| insert_text(event, &any_key))
        .key(Key::Enter, move |event| {
            edit_value(event, &enter, |edit| edit.insert_str("\n"));
        });
    let edits: [(Shortcut, Edit); 6] = [
        (Key::Backspace.into(), TextEdit::backspace),
        (Key::Delete.into(), TextEdit::delete_forward),
        (Key::ArrowLeft.into(), TextEdit::move_left),
        (Key::ArrowRight.into(), TextEdit::move_right),
        (shift(Key::ArrowLeft), TextEdit::shift_left),
        (shift(Key::ArrowRight), TextEdit::shift_right),
    ];
    for (index, (shortcut, edit)) in edits.into_iter().enumerate() {
        let runtime = runtime.clone();
        node = if index < 2 {
            node.key(shortcut, move |event| edit_value(event, &runtime, edit))
        } else {
            node.key(shortcut, move |event| move_caret(event, &runtime, edit))
        };
    }
    node
}

#[derive(Clone, Copy)]
enum LineMove {
    Rows(isize),
    Start,
    End,
}

fn line_keys(mut node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let moves = [
        (Key::ArrowUp, LineMove::Rows(-1)),
        (Key::ArrowDown, LineMove::Rows(1)),
        (Key::PageUp, LineMove::Rows(-PAGE_ROWS)),
        (Key::PageDown, LineMove::Rows(PAGE_ROWS)),
        (Key::Home, LineMove::Start),
        (Key::End, LineMove::End),
    ];
    for (key, line_move) in moves {
        for extend in [false, true] {
            let runtime = runtime.clone();
            let shortcut = if extend { shift(key) } else { key.into() };
            node = node.key(shortcut, move |event: &mut KeyEvent| {
                move_caret(event, &runtime, |edit| move_line(edit, line_move, extend));
            });
        }
    }
    node
}

fn move_line(edit: &mut TextEdit, line_move: LineMove, extend: bool) {
    let lines: Vec<String> = edit.as_str().split('\n').map(str::to_owned).collect();
    let (line, col) = point_at(&lines, edit.caret());
    let last = lines.len().saturating_sub(1);
    let target = match line_move {
        LineMove::Rows(rows) => {
            let next = line.saturating_add_signed(rows).min(last);
            let width = lines.get(next).map_or(0, |text| col_count(text));
            (next, col.min(width))
        }
        LineMove::Start => (line, 0),
        LineMove::End => (line, lines.get(line).map_or(0, |text| col_count(text))),
    };
    let byte = byte_at(&lines, target);
    if extend {
        edit.extend_to(byte);
    } else {
        edit.place_caret(byte);
    }
}

fn shift(key: Key) -> Shortcut {
    Shortcut::new(key, ModifiersState::SHIFT)
}
