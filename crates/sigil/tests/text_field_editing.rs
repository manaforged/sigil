#![allow(missing_docs)]
#![allow(clippy::expect_used)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use reactive as _;
use renderer as _;
use style as _;
use tasks as _;
use wgpu as _;

use hal::{KeyEventInit, PhysicalKey};
use sigil::prelude::*;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::new(40.0, 16.0)
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

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(320.0, 120.0))
        .expect("layout computes");
    layout
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
