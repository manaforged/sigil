//! IME composition tests for the multi-line `text_edit` widget.
//!
//! Mirrors `text_field_ime.rs` — same event-routing shape, same
//! preedit/commit/cancel/disable coverage — adapted for a multi-line
//! buffer: the caret-anchor test proves the published anchor's `top`
//! reflects the caret's *line*, not just its byte offset, and that a
//! commit lands at the right position mid-buffer (not just appended).

#![allow(missing_docs)]
#![allow(clippy::expect_used)]
// The geometry stub uses simple px↔byte casts; precision is irrelevant in tests.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;
use sigil::widgets::text_edit;
use sigil_hal::KeyEventInit;
use sigil_kernel::{NodeType, TextContent};
use sigil_layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil_style::TextStyle;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

/// Deterministic multi-line geometry: 10px per column, 16px per line —
/// `top` is derived from the number of `\n`s before `byte`, so a caret
/// on line 2 reports a distinct `top` from line 0 (real line-aware
/// behaviour, unlike a flat single-line stub).
struct StubGeometry;

impl TextGeometry for StubGeometry {
    fn hit(&self, text: &str, _style: &TextStyle, x: f32) -> usize {
        ((x / 10.0).round().max(0.0) as usize).min(text.len())
    }

    fn caret(&self, text: &str, _style: &TextStyle, byte: usize) -> CaretGeometry {
        let byte = byte.min(text.len());
        let prefix = &text[..byte];
        let line = prefix.matches('\n').count();
        let col_start = prefix.rfind('\n').map_or(0, |i| i + 1);
        CaretGeometry {
            x: (byte - col_start) as f32 * 10.0,
            top: line as f32 * 16.0,
            height: 16.0,
        }
    }
}

#[test]
fn text_edit_preedit_displays_inline() {
    sigil::reactive::Runtime::scope("text-edit-ime-preedit", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );

        assert_eq!(displayed_text(&mut cx, root), "か");
        assert_eq!(
            value.get(),
            "",
            "preedit must not touch the committed buffer"
        );
    });
}

/// Commit inserts at the caret on line 2 of a 3-line buffer, proving
/// (a) mid-buffer insertion (not append), and (b) the published caret
/// anchor's `top` reflects the caret's line, not just x.
#[test]
fn text_edit_commit_lands_at_caret_on_line_two_of_three() {
    sigil::reactive::Runtime::scope("text-edit-ime-line2-commit", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("aaa\nbbb\nccc"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        // Buffer caret starts at end (byte 11); walk it back to byte 4
        // (right after "aaa\n", the start of line 2) with 7 ArrowLefts.
        for _ in 0..7 {
            router.route(&mut cx, &layout, arrow_left());
        }

        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("X".into(), Some((0, 1)))),
        );
        let anchor = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor published while composing on line 2");
        // top = line*16 (StubGeometry) + CONTENT_PADDING_Y (8.0, MEDIUM-3).
        assert!(
            (anchor.origin.y - 24.0).abs() < 0.01,
            "caret on line 2 (0-indexed line 1) must anchor at top=16.0+8.0 padding, got {}",
            anchor.origin.y
        );

        router.route(&mut cx, &layout, ime_event(ImeEvent::Commit("X".into())));

        assert_eq!(
            value.get(),
            "aaa\nXbbb\nccc",
            "commit must insert at the caret mid-buffer, not append"
        );
    });
}

/// W5-IME-TE MEDIUM-3: the anchor must track the OS-reported *compose*
/// cursor within the preedit (`range.1`), not always assume the cursor
/// sits at the end of the preedit text.
#[test]
fn text_edit_anchor_tracks_preedit_cursor_not_just_preedit_end() {
    sigil::reactive::Runtime::scope("text-edit-ime-preedit-cursor", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));

        // Preedit "ab" with the compose cursor after 'a' (byte 1), not
        // at the end (byte 2).
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("ab".into(), Some((0, 1)))),
        );
        let mid_cursor = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor published while composing");

        // Same preedit text, cursor now at the end (byte 2).
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("ab".into(), Some((0, 2)))),
        );
        let end_cursor = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor published while composing");

        assert!(
            (mid_cursor.origin.x - 22.0).abs() < 0.01,
            "cursor after 'a' (byte 1) must anchor at x=1*10+12px padding=22, got {}",
            mid_cursor.origin.x
        );
        assert!(
            (end_cursor.origin.x - 32.0).abs() < 0.01,
            "cursor at end (byte 2) must anchor at x=2*10+12px padding=32, got {}",
            end_cursor.origin.x
        );
        assert!(
            end_cursor.origin.x > mid_cursor.origin.x,
            "moving the compose cursor forward within the preedit must move the anchor forward"
        );
    });
}

/// W5-IME-TE HIGH-1: a regular (non-IME) multibyte-deleting keystroke
/// while a preedit is active must not panic — the buffer shrinks under
/// a caret that was valid for the *old* value but may not be for the
/// new one, and `value`+`caret` publish through separate signals.
#[test]
fn text_edit_multibyte_backspace_during_preedit_does_not_panic() {
    sigil::reactive::Runtime::scope("text-edit-ime-multibyte-backspace", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        // "é" is a 2-byte codepoint; caret at byte 1 is valid for "aé"
        // (between 'a' and 'é') but mid-codepoint for "é" alone.
        let value = signal(String::from("aé"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, arrow_left()); // caret: 3 -> 1

        // Enter composition — the widget now has an active preedit
        // splicing against the caret on every `value` read.
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("X".into(), Some((0, 1)))),
        );

        // A raw (non-IME) Backspace deletes 'a', shrinking value to
        // "é" — must not panic.
        router.route(&mut cx, &layout, backspace());

        assert_eq!(value.get(), "é", "backspace must land on the shrunk buffer");
    });
}

/// W5-IME-TE HIGH-1: a raw (non-IME) keystroke that replaces an active
/// multi-byte selection while a preedit is composing must not panic —
/// the same value/caret atomicity requirement as the backspace case,
/// but through the selection-delete path (`TextEdit::insert_str`'s
/// `delete_selection_range`) rather than plain `backspace`.
#[test]
fn text_edit_selection_replace_during_preedit_does_not_panic() {
    sigil::reactive::Runtime::scope("text-edit-ime-selection-replace", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        // "café" — a 2-byte 'é' — select the whole (multi-byte) buffer
        // via Ctrl+A so replacing it shrinks the buffer by a
        // multi-byte amount under an active preedit, without hand
        // -counting byte offsets.
        let value = signal(String::from("café"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_a());

        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("X".into(), Some((0, 1)))),
        );

        // A raw typed 'z' replaces the selected "café" — must not panic.
        router.route(&mut cx, &layout, type_char("z"));

        let result = value.get();
        assert!(
            result.contains('z'),
            "the typed char must land somewhere, got {result:?}"
        );
        assert!(
            !result.contains('é'),
            "the selected multi-byte text must actually be replaced, got {result:?}"
        );
    });
}

/// W5-IME-TE HIGH-2: an app that writes `value.set(...)` directly after
/// build (already supported per `crates/sigil/tests/text_edit.rs`) must
/// not have that external update silently reverted by a subsequent IME
/// commit operating on a stale internal buffer.
#[test]
fn text_edit_external_value_mutation_survives_ime_commit() {
    sigil::reactive::Runtime::scope("text-edit-ime-external-mutation", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("initial"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        // App replaces the document after build, bypassing the
        // widget's own edit path entirely.
        value.set(String::from("replaced by app"));

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("!".into(), Some((0, 1)))),
        );
        router.route(&mut cx, &layout, ime_event(ImeEvent::Commit("!".into())));

        // Exact insert position depends on the caret-clamp policy
        // (`TextEdit::sync_external`, which preserves the pre-external
        // caret's numeric offset, clamped) — not the load-bearing claim
        // here. What HIGH-2 requires is that the external replacement's
        // characters all survive the commit, in order, and the stale
        // pre-build buffer never resurfaces. Pre-fix, this would have
        // produced "initial!" (the commit clobbering the app's write
        // outright, discarding "replaced by app" entirely).
        let result = value.get();
        assert!(
            !result.contains("initial"),
            "the stale pre-external buffer must not resurface, got {result:?}"
        );
        assert!(
            result.contains('!'),
            "the committed IME text must also land somewhere, got {result:?}"
        );
        assert_eq!(
            result.replacen('!', "", 1),
            "replaced by app",
            "every character of the external replacement must survive, in order, \
             just with the commit spliced in somewhere: got {result:?}"
        );
    });
}

#[test]
fn text_edit_preedit_cancelled_on_focus_loss() {
    sigil::reactive::Runtime::scope("text-edit-ime-focus-loss", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let other = cx.add(
            container()
                .focusable(true)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
        );
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);
        let _ = compute(&mut cx, other);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        assert_eq!(displayed_text(&mut cx, root), "か");

        router.set_focus(&mut cx, Some(other));

        assert_eq!(
            displayed_text(&mut cx, root),
            "",
            "focus loss must cancel preedit — no orphaned span left behind"
        );
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_edit_disabling_mid_composition_clears_preedit_immediately() {
    sigil::reactive::Runtime::scope("text-edit-ime-disable-immediate", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value).disabled(disabled));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        assert_eq!(displayed_text(&mut cx, root), "か");

        // No further ImeEvent at all — disabling the editor itself must
        // clear the stale preedit immediately.
        disabled.set(true);

        assert_eq!(
            displayed_text(&mut cx, root),
            "",
            "disabling the editor must clear a stale preedit without waiting for an ImeEvent"
        );
    });
}

const fn ime_event(event: ImeEvent) -> ShellEvent {
    ShellEvent::Ime(event)
}

fn arrow_left() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::ArrowLeft.into(),
        physical: PhysicalKey::ArrowLeft,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn backspace() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: Key::Backspace.into(),
        physical: PhysicalKey::Backspace,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn ctrl_a() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character("a"),
        physical: PhysicalKey::KeyA,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn type_char(ch: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical: PhysicalKey::KeyZ,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(360.0, 160.0))
        .expect("layout computes");
    layout
}

/// Reads the editor's rendered text. `text_edit` splits its displayed
/// signal into one text node per line via `children_reactive` (B1), so
/// a signal change (preedit spliced in, or cleared) needs a flush
/// before the new child text node exists to read — mirrors the flush
/// pattern in `crates/sigil/tests/text_edit.rs`.
fn displayed_text(cx: &mut Cx, root: NodeId) -> String {
    cx.flush_reactive_children();
    let text_node = find_text_node(cx, root).expect("editor has a text node");
    let Some(node) = cx.get(text_node) else {
        return String::new();
    };
    match &node.kind {
        NodeType::Text {
            content: TextContent::Reactive(signal),
            ..
        } => signal.peek(),
        NodeType::Text {
            content: TextContent::Static(text),
            ..
        } => text.clone(),
        _ => String::new(),
    }
}

fn find_text_node(cx: &Cx, root: NodeId) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none() && matches!(node.kind, NodeType::Text { .. }) {
            found = Some(id);
        }
    });
    found
}
