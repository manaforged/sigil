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
#[path = "main_tests.rs"]
mod tests;
