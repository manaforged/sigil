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
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{
    KeyEventInit, MouseButton, MouseButtons, PhysicalKey, Point, PointerEvent, PointerEventInit,
    PointerKind, PointerPhase,
};
use sigil_layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil_style::TextStyle;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

/// Deterministic geometry: a uniform 10 px per byte. Lets pointer tests assert
/// exact byte hits without depending on real font metrics.
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
fn text_field_undo_reverts_typing() {
    sigil::reactive::Runtime::scope("text-field-editing-undo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        assert_eq!(value.get(), "ab");

        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));

        assert_eq!(value.get(), "a");
    });
}

#[test]
fn text_field_redo_reapplies_undone_edit() {
    sigil::reactive::Runtime::scope("text-field-editing-redo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("x", PhysicalKey::KeyX));
        router.route(&mut cx, &layout, ctrl_key("z", PhysicalKey::KeyZ));
        assert_eq!(value.get(), "");

        router.route(&mut cx, &layout, ctrl_key("y", PhysicalKey::KeyY));

        assert_eq!(value.get(), "x");
    });
}

#[test]
fn text_field_select_all_then_type_replaces_buffer() {
    sigil::reactive::Runtime::scope("text-field-editing-select-all-type", || {
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_key("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("q", PhysicalKey::KeyQ));

        assert_eq!(value.get(), "q");
    });
}

#[test]
fn text_field_pointer_drag_selects_then_type_replaces() {
    // A click-drag across "hello" must select those bytes so the next keystroke
    // replaces them — the externally-observable proof of the full pointer chain
    // (hit-test → place_caret → captured pointer_move → extend_to → selection).
    sigil::reactive::Runtime::scope("text-field-pointer-drag", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        // The text region lays out at global x=13, y=52..68 (verified via the
        // layout tree). Local x = global − 13, then 10 px/byte (StubGeometry):
        // global 14 → byte 0, global 64 → byte 5 (the space after "hello").
        router.route(&mut cx, &layout, pointer_down(14.0, 60.0));
        router.route(&mut cx, &layout, pointer_move(64.0, 60.0));
        router.route(&mut cx, &layout, pointer_up(64.0, 60.0));
        router.route(&mut cx, &layout, type_char("q", PhysicalKey::KeyQ));

        assert_eq!(
            value.get(),
            "q world",
            "drag-selected 'hello' should be replaced by the typed 'q'"
        );
    });
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(320.0, 120.0))
        .expect("layout computes");
    layout
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Move,
    }))
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Up,
    }))
}

fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}

fn ctrl_key(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}
