#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::expect_used)]
// The geometry stub uses simple px↔byte casts; precision is irrelevant in tests.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use sigil::prelude::*;
use sigil_kernel::{NodeType, TextContent};
use sigil_layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil_style::TextStyle;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

struct StubGeometry;

impl TextGeometry for StubGeometry {
    fn hit(&self, text: &str, _style: &TextStyle, x: f32) -> usize {
        ((x / 10.0).round().max(0.0) as usize).min(text.len())
    }

    fn caret(&self, _text: &str, _style: &TextStyle, byte: usize) -> CaretGeometry {
        CaretGeometry {
            x: byte as f32 * 10.0,
            top: 0.0,
            height: 16.0,
        }
    }
}

#[test]
fn text_field_preedit_displays_inline() {
    sigil::reactive::Runtime::scope("text-field-ime-preedit", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );

        assert_eq!(displayed_text(&cx, root), "か");
        assert_eq!(
            value.get(),
            "",
            "preedit must not touch the committed buffer"
        );
    });
}

#[test]
fn text_field_commit_writes_value_and_clears_preedit() {
    sigil::reactive::Runtime::scope("text-field-ime-commit", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        router.route(&mut cx, &layout, ime_event(ImeEvent::Commit("か".into())));

        assert_eq!(value.get(), "か");
        assert_eq!(
            displayed_text(&cx, root),
            "か",
            "preedit span replaced, not duplicated"
        );
    });
}

#[test]
fn text_field_preedit_cancelled_on_focus_loss() {
    sigil::reactive::Runtime::scope("text-field-ime-focus-loss", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let other = cx.add(
            container()
                .focusable(true)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
        );
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);
        let _ = compute(&mut cx, other);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        assert_eq!(displayed_text(&cx, root), "か");

        router.set_focus(&mut cx, Some(other));

        assert_eq!(
            displayed_text(&cx, root),
            "",
            "focus loss must cancel preedit — no orphaned span left behind"
        );
        assert_eq!(value.get(), "");
    });
}

#[test]
fn text_field_disabling_mid_composition_clears_preedit_immediately() {
    sigil::reactive::Runtime::scope("text-field-ime-disable-immediate", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        assert_eq!(displayed_text(&cx, root), "か");

        // No further ImeEvent at all — disabling the field itself must
        // clear the stale preedit immediately .
        disabled.set(true);

        assert_eq!(
            displayed_text(&cx, root),
            "",
            "disabling the field must clear a stale preedit without waiting for an ImeEvent"
        );
    });
}

#[test]
fn text_field_disabled_mid_composition_ignores_commit() {
    sigil::reactive::Runtime::scope("text-field-ime-disable-commit", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::new());
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).disabled(disabled));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        disabled.set(true);

        router.route(&mut cx, &layout, ime_event(ImeEvent::Commit("か".into())));

        assert_eq!(value.get(), "", "a disabled field must not accept a Commit");
        assert_eq!(displayed_text(&cx, root), "");
    });
}

#[test]
fn text_field_ime_caret_anchor_accounts_for_horizontal_scroll() {
    sigil::reactive::Runtime::scope("text-field-ime-scroll-anchor", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal("a".repeat(20));
        let mut cx = Cx::new_root();
        let mut builder = text_field(value);
        let scroll = builder.scroll_offset();
        let root = cx.add(builder);
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ime_event(ImeEvent::Preedit("か".into(), Some((0, 3)))),
        );
        let unscrolled = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor published while composing");

        // The anchor-sync effect reacts to `scroll` directly — no new
        // ImeEvent needed .
        scroll.set(150.0);
        let scrolled = router
            .ime_caret_rect(&cx, &layout)
            .expect("anchor still published after scrolling");

        assert!(
            (unscrolled.origin.x - scrolled.origin.x - 150.0).abs() < 0.01,
            "scrolling the field 150px left must shift the published anchor by the same amount: \
             unscrolled x={} scrolled x={}",
            unscrolled.origin.x,
            scrolled.origin.x,
        );
        assert!(
            scrolled.origin.x >= 0.0,
            "anchor x must stay within the field's visible viewport, never negative"
        );
    });
}

const fn ime_event(event: ImeEvent) -> ShellEvent {
    ShellEvent::Ime(event)
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(320.0, 120.0))
        .expect("layout computes");
    layout
}

fn displayed_text(cx: &Cx, root: NodeId) -> String {
    let text_node = find_text_node(cx, root).expect("field has a text node");
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

fn arrow_left() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(sigil_hal::KeyEventInit {
        key: Key::ArrowLeft.into(),
        physical: sigil_hal::PhysicalKey::ArrowLeft,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn backspace() -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(sigil_hal::KeyEventInit {
        key: Key::Backspace.into(),
        physical: sigil_hal::PhysicalKey::Backspace,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

/// Mirror of a `text_edit` regression: `text_field`'s
/// handlers publish value and caret through separate signal writes, so a
/// raw Backspace during an active preedit can splice mid-codepoint on the
/// transient state. Red before the batch fix, green after.
#[test]
fn text_field_multibyte_backspace_during_preedit_does_not_panic() {
    sigil::reactive::Runtime::scope("text-field-ime-multibyte-backspace", || {
        let value = signal(String::from("a\u{e9}"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
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

        assert_eq!(
            value.get(),
            "\u{e9}",
            "backspace must land on the shrunk buffer"
        );
    });
}
