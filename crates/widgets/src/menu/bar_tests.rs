#![allow(clippy::unwrap_used)]

use super::*;
use sigil_kernel::{Cx, IntoNode};
use sigil_reactive::Runtime;

#[test]
fn bar_builds_one_child_per_menu() {
    Runtime::scope("menu-bar-slots", || {
        let mut cx = Cx::new_root();
        let root = menu_bar()
            .menu("File", [MenuItem::new("New")])
            .menu("Edit", [MenuItem::new("Clear")])
            .build(&mut cx);
        assert_eq!(cx.child_count(root), 2, "File and Edit triggers");
    });
}

#[test]
fn class_is_accepted() {
    Runtime::scope("menu-bar-class", || {
        let mut cx = Cx::new_root();
        let root = menu_bar()
            .class("bg-surface-hover px-4")
            .menu("File", [MenuItem::new("Open")])
            .build(&mut cx);
        assert!(cx.child_count(root) >= 1);
    });
}
