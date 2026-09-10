#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("log-tailer-smoke", |cx| {
        let state = AppState::new();
        let root = cx.add(app_tree(state));
        assert!(cx.child_count(root) > 0, "root must have children");
    });
}
