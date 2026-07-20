//! Windowed style proof — static + hover-reactive panes via `sigil::launch`.
//!
//! The hover pane uses `interactive()` (5/5 Wave 1 T2): no pointer math,
//! no manual hit-testing, no `style_reactive` boilerplate — the fluent
//! `.hover()` closure paints directly.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    // 720x480 logical pixels; physical size depends on monitor scale
    // (1080x720 at 1.5x, 1440x960 at 2x).
    sigil::launch(("Sigil - hello-style", 720, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(720.0, 480.0)
                    .gap(16.0),
            )
            .child(top_row())
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(688.0, 128.0))
                    .class(classes!("p-4 bg-accent rounded-md")),
            ),
    )
}

fn top_row() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(688.0, 304.0).gap(16.0))
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
                .class(classes!("p-8 bg-accent rounded-md")),
        )
        .child(
            interactive(
                container()
                    .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
                    .class(classes!("p-8 bg-accent rounded-md")),
            )
            .hover(|s| s.bg("accent-hover")),
        )
}
