#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("data-table-smoke", |cx| {
        let root = cx.add(app_tree());
        assert!(cx.child_count(root) > 0, "root must have children");
    });
}
