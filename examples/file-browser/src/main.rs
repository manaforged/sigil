pub mod files;
pub mod left_pane;
pub mod right_pane;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil — file-browser", 1_024, 640), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let files = files::inventory();
    let selected_index = signal(0_usize);

    let left = left_pane::build(&files, selected_index);
    let right = right_pane::build(files, selected_index);

    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(1_024.0, 640.0)
                    .gap(0.0),
            )
            .class(classes!("bg-surface"))
            .child(
                text("file-browser")
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
