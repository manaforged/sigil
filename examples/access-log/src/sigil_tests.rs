#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("access-log-sigil", |cx| {
        let root = build(cx, 200);
        assert!(cx.child_count(root) > 0);
    });
}
