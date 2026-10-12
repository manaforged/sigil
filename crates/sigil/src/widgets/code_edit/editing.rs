use crate::clipboard::clipboard;
use std::rc::Rc;

use crate::events::{Key, NodeEventsExt};
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, batch, effect, signal};
use crate::widgets::grapheme::col_count;
use crate::widgets::text_edit::{
    KeyProfile, LineStep, TextEdit, TextEditRuntime, TextEditState, attach_a11y_edit,
    attach_ime_handlers, clipboard_keys, edit_keys, edit_value, history_keys, move_caret,
    noop_change, word_keys,
};

use super::SelectionAnchor;
use super::a11y::{byte_at, point_at};

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
            value.set(joined);
            caret.set(at);
            selection.set(anchor);
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
            model.lines.set(lines);
        });
    });
}

pub(super) fn attach(
    node: NodeBuilder,
    runtime: &TextEditRuntime,
    viewport: Signal<(f32, f32)>,
    row_h: f32,
) -> NodeBuilder {
    let (edge, line) = (runtime.clone(), runtime.clone());
    let profile = KeyProfile {
        edge: Rc::new(move |event, end, extend| {
            let target = if end { LineMove::End } else { LineMove::Start };
            move_caret(event, &edge, |edit| move_line(edit, target, extend));
        }),
        line: Some(Rc::new(move |event, step: LineStep, extend| {
            let rows = step.rows(viewport.peek().1, row_h);
            move_caret(event, &line, |edit| {
                move_line(edit, LineMove::Rows(rows), extend);
            });
        })),
    };
    let enter = runtime.clone();
    let node = edit_keys(node, runtime, &profile).key(Key::Enter, move |event| {
        edit_value(event, &enter, |edit| edit.insert_str("\n"));
    });
    let node = clipboard_keys(node, runtime);
    let node = word_keys(node, runtime);
    let node = history_keys(node, runtime.clone());
    let node = attach_a11y_edit(node, runtime.clone());
    attach_ime_handlers(node, runtime)
}

#[derive(Clone, Copy)]
enum LineMove {
    Rows(isize),
    Start,
    End,
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
