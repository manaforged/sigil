use std::cell::Cell;
use std::rc::Rc;

use sigil_events::prelude::{ImeEvent, NodeEventsExt};
use sigil_kernel::NodeBuilder;
use sigil_layout::{CaretGeometry, with_text_geometry};
use sigil_reactive::{Signal, effect, signal};

use super::handlers::{TextFieldRuntime, commit, write_value};

#[derive(Clone)]
pub(super) struct ImeState {
    pub(super) preedit: Signal<String>,
    caret_anchor: Rc<Cell<Option<CaretGeometry>>>,
}

impl ImeState {
    pub(super) fn new() -> Self {
        Self {
            preedit: signal(String::new()),
            caret_anchor: Rc::new(Cell::new(None)),
        }
    }
}

pub(super) fn attach_ime_handlers(node: NodeBuilder, runtime: &TextFieldRuntime) -> NodeBuilder {
    let anchor = Rc::clone(&runtime.ime.caret_anchor);
    let event_rt = runtime.clone();
    let blur_rt = runtime.clone();
    let eligible_rt = runtime.clone();
    install_disable_clears_preedit(runtime);
    install_caret_anchor_sync(runtime);
    node.on_ime(move |event| on_ime_event(event, &event_rt))
        .focus_out(move || blur_rt.ime.preedit.set(String::new()))
        .ime_eligible(move || !eligible_rt.disabled())
        .ime_caret(anchor)
}

fn on_ime_event(event: &ImeEvent, runtime: &TextFieldRuntime) {
    if matches!(event, ImeEvent::Disabled) {
        runtime.ime.preedit.set(String::new());
        return;
    }
    if runtime.disabled() {
        return;
    }
    match event {
        ImeEvent::Preedit(text, _range) => runtime.ime.preedit.set(text.to_string()),
        ImeEvent::Commit(text) => apply_commit(runtime, text),
        ImeEvent::Enabled | ImeEvent::Disabled => {}
    }
}

fn apply_commit(runtime: &TextFieldRuntime, text: &str) {
    runtime.ime.preedit.set(String::new());
    if text.is_empty() {
        return;
    }
    let mut edit = runtime.edit.borrow_mut();
    edit.insert_str(text);
    write_value(runtime.value, edit.as_str());
    (runtime.change)(edit.as_str());
    commit(runtime, &edit);
}

fn install_disable_clears_preedit(runtime: &TextFieldRuntime) {
    let Some(disabled) = runtime.disabled else {
        return;
    };
    let preedit = runtime.ime.preedit;
    effect(move || {
        if disabled.get() && !preedit.peek().is_empty() {
            preedit.set(String::new());
        }
    });
}

fn install_caret_anchor_sync(runtime: &TextFieldRuntime) {
    let preedit = runtime.ime.preedit;
    let disabled = runtime.disabled;
    let value = runtime.value;
    let caret = runtime.caret;
    let scroll = runtime.scroll;
    let text_style = runtime.text_style;
    let anchor = Rc::clone(&runtime.ime.caret_anchor);
    effect(move || {
        let composing = !preedit.get().is_empty();
        let is_disabled = disabled.is_some_and(Signal::get);
        if !composing || is_disabled {
            anchor.set(None);
            return;
        }
        let text = value.get();
        let caret_byte = caret.get();
        let scroll_px = scroll.get();
        let style = text_style.get();
        let rect = with_text_geometry(|engine| {
            let pos = engine.caret(&text, &style, caret_byte);
            CaretGeometry {
                x: (pos.x - scroll_px).max(0.0),
                top: pos.top,
                height: pos.height,
            }
        });
        anchor.set(rect);
    });
}
