#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

#[path = "events_support/input.rs"]
mod input;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::{pointer_down, pointer_move, pointer_up, type_char};
use sigil::hal::{KeyEventInit, PhysicalKey};
use sigil::layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil::prelude::*;
use sigil::style::TextStyle;
use widget_support::compute;

const VIEWPORT: Size = Size::fixed(320.0, 120.0);

struct StubGeometry;

impl TextGeometry for StubGeometry {
    fn hit(
        &self,
        text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        point: sigil::hal::Point,
    ) -> sigil::hal::HitPosition {
        sigil::hal::HitPosition {
            byte_index: ((point.x / 10.0).round().max(0.0) as usize).min(text.len()),
            affinity: sigil::hal::TextAffinity::Downstream,
        }
    }

    fn caret(
        &self,
        _text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        position: sigil::hal::HitPosition,
    ) -> CaretGeometry {
        let byte = position.byte_index;
        CaretGeometry {
            x: byte as f32 * 10.0,
            top: 0.0,
            height: 16.0,
        }
    }

    fn selection_rects(
        &self,
        _text: &str,
        _style: &TextStyle,
        _width: Option<f32>,
        _selection: (sigil::hal::HitPosition, sigil::hal::HitPosition),
    ) -> Vec<sigil::hal::Rect> {
        Vec::new()
    }
}

#[test]
fn text_field_undo_reverts_typing() {
    sigil::reactive::Runtime::scope("text-field-editing-undo", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, ctrl_key("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("q", PhysicalKey::KeyQ));

        assert_eq!(value.get(), "q");
    });
}

#[test]
fn text_field_pointer_drag_selects_then_type_replaces() {
    sigil::reactive::Runtime::scope("text-field-pointer-drag", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("hello world"));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, pointer_down(14.0, 18.0));
        router.route(&mut cx, &layout, pointer_move(64.0, 18.0));
        router.route(&mut cx, &layout, pointer_up(64.0, 18.0));
        router.route(&mut cx, &layout, type_char("q", PhysicalKey::KeyQ));

        assert_eq!(
            value.get(),
            "q world",
            "drag-selected 'hello' should be replaced by the typed 'q'"
        );
    });
}

#[test]
fn text_field_external_write_resyncs_edit_buffer() {
    sigil::reactive::Runtime::scope("text-field-editing-external-write", || {
        let value = signal(String::new());
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, type_char("a", PhysicalKey::KeyA));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        assert_eq!(value.get(), "ab");

        value.set(String::from("cleared"));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: Key::Backspace.into(),
                physical: PhysicalKey::Backspace,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );

        assert_eq!(
            value.get(),
            "ceared",
            "backspace must edit the externally-written buffer, not the stale 'ab'"
        );
    });
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
