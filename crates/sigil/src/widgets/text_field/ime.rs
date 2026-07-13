//! IME composition wiring for `text_field` (DR-051 Slice 2).
//!
//! Preedit state lives widget-local (a `Signal` the widget owns) rather
//! than the DR's full `Entity<Composition>` — the router delivers
//! `ImeEvent`s to the focused node's `on_ime` handler exactly the way
//! `KeyEvent::text` already reaches `TextFieldRuntime::value` today.
//! Preedit renders inline (spliced into the displayed string at the
//! caret) with no distinct visual treatment: `crates/text` has no
//! decoration/underline support to hang an in-progress-composition cue
//! on (confirmed by grep — no `underline`/`Decoration` type anywhere in
//! `style`/`text`), so this ships same-color insert. Follow-up: add
//! underline decoration support, then style the preedit span.

use std::cell::Cell;
use std::rc::Rc;

use events::prelude::{ImeEvent, NodeEventsExt};
use kernel::NodeBuilder;
use layout::{CaretGeometry, with_text_geometry};
use reactive::{Signal, effect, signal};
use style::TextStyle;

use super::handlers::{TextFieldRuntime, commit, write_value};

/// Widget-local IME composition state: the in-progress preedit text
/// (spliced into the displayed string at the caret while composing) and
/// the caret-anchor rect published to the router for
/// `Shell::set_ime_cursor_area`.
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

/// Attach the field's IME opt-in: an `on_ime` handler that writes
/// preedit/commit into the runtime, a `focus_out` handler that clears
/// any orphaned preedit span on blur, an `.ime_eligible(...)` query so
/// the router's dispatch gate and `focus_wants_ime` see this field's
/// *current* disabled state (not just handler registration), and two
/// reactive effects that keep preedit/caret-anchor honest across
/// disable/scroll/caret changes without waiting for the next `ImeEvent`.
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

/// Lifecycle/cancel events (`Disabled`) always process, even while the
/// field is disabled — a disabled field must still let go of an
/// in-flight composition instead of leaving a stale preedit rendered
/// (W5-IME HIGH-2). Content-mutating events (`Enabled`/`Preedit`/
/// `Commit`) still no-op while disabled, matching raw `KeyEvent::text`.
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
        // `Enabled` has no content to apply; `Disabled` is handled above
        // (kept here only for match exhaustiveness).
        ImeEvent::Enabled | ImeEvent::Disabled => {}
    }
}

/// Replace the preedit span with the committed text at the caret —
/// the same `TextEdit::insert_str` path raw `KeyEvent::text` already
/// uses, so undo history and the `change` callback stay unified.
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

/// Watches the field's own `disabled` signal and clears a stale preedit
/// the instant it flips true — doesn't wait for an `ImeEvent::Disabled`
/// that may never arrive if the app disabled the field independent of
/// the OS IME (W5-IME HIGH-2).
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

/// Keeps the published caret-anchor rect in the same scrolled,
/// rendered-space coordinates the text track actually paints in
/// (subtracting `runtime.scroll`, matching `selection.rs`'s caret/
/// selection overlays), refreshing on every caret/scroll/value/disabled
/// change instead of only at the `Preedit` event that started
/// composing. Publishes `None` once composition ends or the field is
/// disabled, so a stale rect never survives past its session
/// (W5-IME MEDIUM-3).
fn install_caret_anchor_sync(runtime: &TextFieldRuntime) {
    let preedit = runtime.ime.preedit;
    let disabled = runtime.disabled;
    let value = runtime.value;
    let caret = runtime.caret;
    let scroll = runtime.scroll;
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
        let rect = with_text_geometry(|engine| {
            let pos = engine.caret(&text, &TextStyle::default(), caret_byte);
            CaretGeometry {
                x: (pos.x - scroll_px).max(0.0),
                top: pos.top,
                height: pos.height,
            }
        });
        anchor.set(rect);
    });
}
