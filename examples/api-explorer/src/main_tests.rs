use super::*;
use sigil::testing::with_cx;

#[test]
fn smoke_app_tree_builds_without_panic() {
    with_cx("api-explorer-smoke", |cx| {
        let root = app_tree(cx);
        assert!(
            cx.get(root).is_some(),
            "app_tree must return a non-null NodeId"
        );
    });
}

#[test]
fn catalog_is_non_empty_and_paths_are_unique() {
    let cat = endpoints::catalog();
    assert!(cat.len() >= 10, "expected ≥10 mock endpoints");
    let mut paths: Vec<(&str, &str)> = cat.iter().map(|e| (e.method.0, e.path)).collect();
    paths.sort_unstable();
    let len_before = paths.len();
    paths.dedup();
    assert_eq!(
        paths.len(),
        len_before,
        "method+path pairs must be unique in the catalog"
    );
}

#[test]
fn header_block_includes_status_and_latency() {
    let cat = endpoints::catalog();
    let first = cat.first().expect("catalog non-empty");
    let block = endpoints::header_block(first);
    assert!(block.starts_with("HTTP/2 "));
    assert!(block.contains("x-response-time-ms:"));
    assert!(block.contains(&first.latency_ms.to_string()));
}
