#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use sigil::input_glyphs::{
    InputAction, InputDevice, current_input_device, glyph, glyph_for, glyph_id, reset_for_tests,
};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kernel::{NodeType, TextContent};

#[test]
fn glyph_id_covers_every_action_on_every_device() {
    let actions = [
        InputAction::Confirm,
        InputAction::Cancel,
        InputAction::Menu,
        InputAction::Up,
        InputAction::Down,
        InputAction::Left,
        InputAction::Right,
        InputAction::Jump,
        InputAction::Attack,
        InputAction::Interact,
    ];
    let devices = [
        InputDevice::Xbox,
        InputDevice::Playstation,
        InputDevice::Switch,
        InputDevice::Keyboard,
    ];
    for action in actions {
        for device in devices {
            assert!(
                !glyph_id(action, device).is_empty(),
                "missing glyph for ({action:?}, {device:?})"
            );
        }
    }
}

#[test]
fn glyph_for_emits_reactive_text_with_correct_static_glyph() {
    reset_for_tests();
    Runtime::scope("glyph_for_static", || {
        let mut cx = Cx::new_root();
        let root = cx.add(glyph_for(InputAction::Confirm, InputDevice::Playstation));
        let child = cx.first_child(root).expect("fragment has child text");
        let node = cx.get(child).expect("text node present");
        match &node.kind {
            NodeType::Text {
                content: TextContent::Static(value),
                ..
            } => assert_eq!(value, "✕"),
            other => panic!("expected static text, got {other:?}"),
        }
    });
}

#[test]
fn glyph_emits_reactive_text_bound_to_current_input_device() {
    reset_for_tests();
    Runtime::scope("glyph_reactive_kind", || {
        let mut cx = Cx::new_root();
        let root = cx.add(glyph(InputAction::Confirm));
        let child = cx.first_child(root).expect("fragment has child text");
        let node = cx.get(child).expect("text node present");
        assert!(
            matches!(
                &node.kind,
                NodeType::Text {
                    content: TextContent::Reactive(_),
                    ..
                }
            ),
            "glyph() must emit reactive text so hot-swap re-renders without invalidation"
        );
    });
}

#[test]
fn hot_swap_re_renders_glyph_when_device_signal_changes() {
    reset_for_tests();
    Runtime::scope("glyph_hot_swap", || {
        let mut cx = Cx::new_root();
        let root = cx.add(glyph(InputAction::Confirm));
        let child = cx.first_child(root).expect("fragment has child text");

        let device = current_input_device();
        let signal = match &cx.get(child).expect("text node").kind {
            NodeType::Text {
                content: TextContent::Reactive(sig),
                ..
            } => *sig,
            other => panic!("expected reactive text, got {other:?}"),
        };

        assert_eq!(signal.get(), "Enter", "default device is Keyboard");
        device.set(InputDevice::Xbox);
        assert_eq!(signal.get(), "A", "hot-swap to Xbox renders A");
        device.set(InputDevice::Playstation);
        assert_eq!(signal.get(), "✕", "hot-swap to PlayStation renders ✕");
        device.set(InputDevice::Switch);
        assert_eq!(signal.get(), "B", "hot-swap to Switch renders B");
        device.set(InputDevice::Keyboard);
        assert_eq!(
            signal.get(),
            "Enter",
            "hot-swap back to Keyboard renders Enter"
        );
    });
}

#[test]
fn current_input_device_returns_same_signal_within_scope() {
    reset_for_tests();
    Runtime::scope("glyph_signal_identity", || {
        let a = current_input_device();
        let b = current_input_device();
        a.set(InputDevice::Switch);
        assert_eq!(
            b.get(),
            InputDevice::Switch,
            "second handle observes first handle's write"
        );
    });
}
