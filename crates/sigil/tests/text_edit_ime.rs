#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::expect_used)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use sigil::hal::KeyEventInit;
use sigil::kernel::{NodeType, TextContent};
use sigil::layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil::prelude::*;
use sigil::style::TextStyle;
use sigil::widgets::text_edit;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

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

        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("ab".into(), Some((0, 1)))),
        );
        let mid_cursor = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor published while composing");

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

#[test]
fn text_edit_multibyte_backspace_during_preedit_does_not_panic() {
    sigil::reactive::Runtime::scope("text-edit-ime-multibyte-backspace", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("aé"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, arrow_left());

        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("X".into(), Some((0, 1)))),
        );

        router.route(&mut cx, &layout, backspace());

        assert_eq!(value.get(), "é", "backspace must land on the shrunk buffer");
    });
}

#[test]
fn text_edit_selection_replace_during_preedit_does_not_panic() {
    sigil::reactive::Runtime::scope("text-edit-ime-selection-replace", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
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

#[test]
fn text_edit_external_value_mutation_survives_ime_commit() {
    sigil::reactive::Runtime::scope("text-edit-ime-external-mutation", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("initial"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(value));
        let layout = compute(&mut cx, root);

        value.set(String::from("replaced by app"));

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("!".into(), Some((0, 1)))),
        );
        router.route(&mut cx, &layout, ime_event(ImeEvent::Commit("!".into())));

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
