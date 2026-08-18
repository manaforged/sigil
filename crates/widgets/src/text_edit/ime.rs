//! IME composition wiring for `text_edit` (W5-IME-TE), mirroring
//! `text_field::ime` (DR-051 Slice 2) for the multi-line buffer.
//!
//! Preedit state lives widget-local exactly as it does for `text_field`:
//! a `Signal<String>` the widget owns, spliced into the displayed
//! buffer at the caret for display only. Commit inserts through the
//! same `TextEdit::insert_str` path a typed character does — a
//! multi-line commit (a preedit or commit string containing `\n`)
//! reflows through the existing `children_reactive` line list exactly
//! like a typed `Enter` does; no separate handling needed.
//!
//! The published IME caret anchor differs from `text_field` in one way:
//! `text_edit` has no horizontal scroll signal to subtract, and the
//! `top` offset for a multi-line buffer's caret naturally comes from
//! the text engine's paragraph layout (a `\n` is a hard line break),
//! so passing the composed text into `engine.caret(...)` already
//! yields the right line's `y` offset — no extra line-counting needed
//! here. It also composes against the *preedit* cursor (not just the
//! base caret) and adds the widget's content padding, so the anchor
//! tracks where the glyphs actually paint mid-composition (the reviewer
//! adversarial finding [W5-IME-TE] MEDIUM-3).
//!
//! Ceiling: `TextGeometry::caret` has no wrap-width parameter
//! (`EngineGeometry::caret` passes `None`). A preedit that wraps in
//! the editor still anchors against an unconstrained line.

use std::cell::Cell;
use std::rc::Rc;

use sigil_events::prelude::{ImeEvent, NodeEventsExt};
use sigil_kernel::NodeBuilder;
use sigil_layout::{CaretGeometry, with_text_geometry};
use sigil_reactive::{Signal, batch, effect, signal};
use sigil_style::TextStyle;

use super::handlers::{TextEditRuntime, commit, write_value};
use super::render::{CONTENT_PADDING_X, CONTENT_PADDING_Y, floor_char_boundary, splice_preedit};

/// Widget-local IME composition state: the in-progress preedit text
/// (spliced into the displayed buffer at the caret while composing),
/// its cursor byte range as reported by the OS (used to anchor the
/// candidate window at the *compose* cursor, not just the base
/// caret), and the caret-anchor rect published to the router for
/// `Shell::set_ime_cursor_area`.
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

/// Attach the editor's IME opt-in: an `on_ime` handler that writes
/// preedit/commit into the runtime, a `focus_out` handler that clears
/// any orphaned preedit span on blur, an `.ime_eligible(...)` query,
/// and two reactive effects that keep preedit/caret-anchor honest
/// across disable/caret changes without waiting for the next
/// `ImeEvent`. Mirrors `text_field::ime::attach_ime_handlers`.
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

/// Clears preedit text + cursor as one atomic unit — every path that
/// abandons a composition (focus loss, disable, `Disabled` event) goes
/// through this instead of setting `preedit` alone, so a reader never
/// observes `preedit == ""` paired with a stale `preedit_cursor`.
fn clear_preedit(runtime: &TextEditRuntime) {
    batch(|| {
        runtime.ime.preedit.set(String::new());
        runtime.ime.preedit_cursor.set(None);
    });
}

/// Lifecycle/cancel events (`Disabled`) always process, even while the
/// editor is disabled — a disabled editor must still let go of an
/// in-flight composition instead of leaving a stale preedit rendered
/// (mirrors the `text_field` W5-IME HIGH-2 finding). Content-mutating
/// events (`Enabled`/`Preedit`/`Commit`) still no-op while disabled,
/// matching raw `KeyEvent::text`.
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
        // `Enabled` has no content to apply; `Disabled` is handled above
        // (kept here only for match exhaustiveness).
        ImeEvent::Enabled | ImeEvent::Disabled => {}
    }
}

/// Replace the preedit span with the committed text at the caret — the
/// same `TextEdit::insert_str` path raw `KeyEvent::text` already uses,
/// so undo history and the `change` callback stay unified.
///
/// `runtime.edit_mut()` resyncs the internal buffer against the bound
/// `value` signal first, so an app that wrote `value.set(...)` directly
/// while this widget existed is not silently clobbered by a stale
/// internal buffer (the reviewer adversarial finding [W5-IME-TE] HIGH-2). The
/// whole mutation is one `batch` so every dependent effect (the
/// preedit-splice `derived_eq`, the caret-anchor sync below) always
/// observes the post-commit `(value, caret)` pair together, never a
/// half-updated one (HIGH-1, same root cause as `sync_value`'s batch —
/// see `super::handlers`).
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

/// Watches the editor's own `disabled` signal and clears a stale
/// preedit the instant it flips true — doesn't wait for an
/// `ImeEvent::Disabled` that may never arrive if the app disabled the
/// editor independent of the OS IME.
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

/// Keeps the published caret-anchor rect current across caret/preedit/
/// value/disabled changes instead of only at the `Preedit` event that
/// started composing. Publishes `None` once composition ends or the
/// editor is disabled, so a stale rect never survives past its
/// session.
///
/// Anchors at the *compose* cursor, not just the base caret: composes
/// the full displayed text (`splice_preedit`, same as rendering) and
/// resolves geometry at `base_caret + preedit_cursor`, where
/// `preedit_cursor` is the end of the OS-reported preedit range
/// (falls back to the end of the preedit text when the OS sends no
/// range) — so a preedit like `"a\nb"` anchors on whichever line the
/// compose cursor is actually on, not the line the composition
/// started on (MEDIUM-3). Adds `CONTENT_PADDING_X/Y` so the anchor
/// matches the content offset rendering actually uses (verified
/// empirically — see the const docs in `super::render`).
///
/// No scroll subtraction: `text_edit` has no scroll signal
/// (pre-existing gap, tracked in docs/followups.md, unrelated to IME).
/// No line-width constraint: see the module-level ceiling note.
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
