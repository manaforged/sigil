#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use sigil_kernel::{AccessibleName, Cx, Role};
use sigil_reactive::Runtime;

#[test]
fn icon_button_is_a_ghost_button() {
    Runtime::scope("icon-button-role", || {
        let mut cx = Cx::new_root();
        let id = icon_button("\u{25B6}", || {})
            .accessible("Play")
            .build(&mut cx);
        let node = cx.get(id).expect("button");
        assert_eq!(node.a11y.role, Role::Button);
        match &node.a11y.name {
            AccessibleName::Static(name) => assert_eq!(name, "Play"),
            other => panic!("expected static accessible name, got {other:?}"),
        }
    });
}

#[test]
fn icon_button_defaults_accessible_name_to_glyph() {
    Runtime::scope("icon-button-glyph-name", || {
        let mut cx = Cx::new_root();
        let id = icon_button("\u{25B6}", || {}).build(&mut cx);
        let node = cx.get(id).expect("button");
        match &node.a11y.name {
            AccessibleName::Static(name) => assert_eq!(name, "\u{25B6}"),
            other => panic!("expected static name, got {other:?}"),
        }
    });
}

#[test]
fn pending_button_starts_disabled() {
    Runtime::scope("button-pending", || {
        let busy = sigil_reactive::signal(true);
        let mut cx = Cx::new_root();
        let id = button("Save").pending(busy).build(&mut cx);
        let node = cx.get(id).expect("button");
        assert!(!node.a11y.focusable, "pending button is not focusable");
    });
}
