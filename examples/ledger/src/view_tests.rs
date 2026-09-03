#![allow(clippy::indexing_slicing, clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_and_rebuilds_on_theme_toggle() {
    with_cx("ledger-smoke", |cx| {
        let app = App::new(300);
        let root = cx.add(app_tree(cx, &app));
        cx.flush_reactive_children();
        let frame = cx.nth_child(root, 0).unwrap();
        assert_eq!(cx.child_count(frame), 3);
        app.toggle_theme();
        cx.flush_reactive_children();
        let rebuilt = cx.nth_child(root, 0).unwrap();
        assert_ne!(frame, rebuilt);
        assert_eq!(cx.child_count(rebuilt), 3);
    });
}
