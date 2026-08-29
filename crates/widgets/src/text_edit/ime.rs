use std::cell::Cell;
use std::rc::Rc;

use sigil_events::prelude::{ImeEvent, NodeEventsExt};
use sigil_kernel::NodeBuilder;
use sigil_layout::{CaretGeometry, with_text_geometry};
use sigil_reactive::{Signal, batch, effect, signal};
use sigil_style::TextStyle;

use super::handlers::{TextEditRuntime, commit, write_value};
use super::render::{CONTENT_PADDING_X, CONTENT_PADDING_Y, floor_char_boundary, splice_preedit};

#[derive(Clone)]
pub(super) struct ImeState {
    pub(super) preedit: Signal<String>,
    preedit_cursor: Signal<Option<(usize, usize)>>,
    caret_anchor: Rc<Cell<Option<CaretGeometry>>>,
}

impl ImeState {
    pub(super) fn new() -> Self {
        Self {
            preedit: signal(String::new()),
            preedit_cursor: signal(None),
            caret_anchor: Rc::new(Cell::new(None)),
        }
    }
}

pub(super) fn attach_ime_handlers(node: NodeBuilder, runtime: &TextEditRuntime) -> NodeBuilder {
    let anchor = Rc::clone(&runtime.ime.caret_anchor);
    let event_rt = runtime.clone();
    let blur_rt = runtime.clone();
    let eligible_rt = runtime.clone();
    install_disable_clears_preedit(runtime);
    install_caret_anchor_sync(runtime);
    node.on_ime(move |event| on_ime_event(event, &event_rt))
        .focus_out(move || clear_preedit(&blur_rt))
        .ime_eligible(move || !eligible_rt.disabled())
        .ime_caret(anchor)
}

fn clear_preedit(runtime: &TextEditRuntime) {
    batch(|| {
        runtime.ime.preedit.set(String::new());
        runtime.ime.preedit_cursor.set(None);
    });
}

fn on_ime_event(event: &ImeEvent, runtime: &TextEditRuntime) {
    if matches!(event, ImeEvent::Disabled) {
        clear_preedit(runtime);
        return;
    }
    if runtime.disabled() {
        return;
    }
    match event {
        ImeEvent::Preedit(text, range) => {
            batch(|| {
                runtime.ime.preedit.set(text.to_string());
                runtime.ime.preedit_cursor.set(*range);
            });
        }
        ImeEvent::Commit(text) => apply_commit(runtime, text),
        ImeEvent::Enabled | ImeEvent::Disabled => {}
    }
}

fn apply_commit(runtime: &TextEditRuntime, text: &str) {
    batch(|| {
        runtime.ime.preedit.set(String::new());
        runtime.ime.preedit_cursor.set(None);
        if text.is_empty() {
            return;
        }
        let mut edit = runtime.edit_mut();
        edit.insert_str(text);
        write_value(runtime.value, edit.as_str());
        (runtime.change)(edit.as_str());
        commit(runtime, &edit);
    });
}

fn install_disable_clears_preedit(runtime: &TextEditRuntime) {
    let Some(disabled) = runtime.disabled else {
        return;
    };
    let disabled_rt = runtime.clone();
    let preedit = runtime.ime.preedit;
    effect(move || {
        if disabled.get() && !preedit.peek().is_empty() {
            clear_preedit(&disabled_rt);
        }
    });
}

fn install_caret_anchor_sync(runtime: &TextEditRuntime) {
    let preedit = runtime.ime.preedit;
    let preedit_cursor = runtime.ime.preedit_cursor;
    let disabled = runtime.disabled;
    let value = runtime.value;
    let caret = runtime.caret;
    let anchor = Rc::clone(&runtime.ime.caret_anchor);
    effect(move || {
        let preedit_text = preedit.get();
        let composing = !preedit_text.is_empty();
        let is_disabled = disabled.is_some_and(Signal::get);
        if !composing || is_disabled {
            anchor.set(None);
            return;
        }
        let base_value = value.get();
        let base_caret = floor_char_boundary(&base_value, caret.get());
        let composed = splice_preedit(&base_value, base_caret, &preedit_text);
        let cursor_in_preedit = preedit_cursor.get().map_or(preedit_text.len(), |(_, end)| {
            floor_char_boundary(&preedit_text, end)
        });
        let effective_cursor = base_caret + cursor_in_preedit;
        let rect = with_text_geometry(|engine| {
            let pos = engine.caret(&composed, &TextStyle::default(), effective_cursor);
            CaretGeometry {
                x: (pos.x + CONTENT_PADDING_X).max(0.0),
                top: pos.top + CONTENT_PADDING_Y,
                height: pos.height,
            }
        });
        anchor.set(rect);
    });
}
