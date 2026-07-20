//! api-explorer dogfood example.
//!
//! Multi-pane mock REST API explorer. Left pane is a `virtual_list` of
//! endpoint paths; a horizontal `splitter` separates it from the right
//! pane, which is a `tabs` widget with three routed panels
//! (Request / Response / Headers). Clicking a row in the left pane
//! re-routes the right pane to that endpoint's data.
//!
//! Everything is mocked — see `endpoints::catalog` — so the example is
//! deterministic and works as a `cargo test` smoke target.

pub mod endpoints;
pub mod left_pane;
pub mod right_pane;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil — api-explorer", 1_024, 640), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let catalog = endpoints::catalog();
    let selected_index = signal(0_usize);

    let left = left_pane::build(&catalog, selected_index);
    let right = right_pane::build(catalog, selected_index);

    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(1_024.0, 640.0)
                    .gap(0.0),
            )
            .class(classes!("bg-surface"))
            .child(
                text("api-explorer")
                    .layout(LayoutStyle::block().width(1_024.0))
                    .class(classes!("text-on-surface text-lg font-bold px-4 py-2")),
            )
            .child(
                splitter(SplitterOrientation::Horizontal)
                    .child(left, 360.0)
                    .child(right, 656.0)
                    .min_size(0, 200.0)
                    .min_size(1, 320.0),
            ),
    )
}

#[cfg(test)]
mod tests {
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
}
