use super::*;
use sigil::testing::with_cx;

#[test]
fn smoke_app_tree_builds_without_panic() {
    with_cx("file-browser-smoke", |cx| {
        let root = app_tree(cx);
        assert!(
            cx.get(root).is_some(),
            "app_tree must return a non-null NodeId"
        );
    });
}
