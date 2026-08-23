#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn app_menu_builds() {
    with_cx("todo-menu-bar", |cx| {
        let state = TodoState::seed();
        let root = cx.add(app_menu(state));
        assert_eq!(cx.child_count(root), 2, "File and Edit menus");
    });
}
