#![allow(clippy::unwrap_used)]

use super::*;
use crate::kernel::{Cx, IntoNode};
use crate::reactive::Runtime;

#[test]
#[allow(
    clippy::suboptimal_flops,
    clippy::float_cmp,
    reason = "exact comparison is the contract (regression pin)"
)]
fn trigger_width_is_pad_plus_count_times_em() {
    assert_eq!(super::trigger_width("File"), 16.0 + 4.0 * 8.0);
    assert_eq!(super::trigger_width(""), 16.0);
}

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
fn paint_is_accepted() {
    Runtime::scope("menu-bar-paint", || {
        let mut cx = Cx::new_root();
        let root = menu_bar()
            .bg(Token::SurfaceHover)
            .menu("File", [MenuItem::new("Open")])
            .build(&mut cx);
        assert!(cx.child_count(root) >= 1);
    });
}
