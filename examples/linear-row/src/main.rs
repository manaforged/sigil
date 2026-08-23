//! Linear-style issue row from one class string.
//!
//! No `LayoutStyle` on the row. Hover lives on `interactive().class`.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - linear-row", 640, 120), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(640.0, 120.0))
            .class("bg-canvas p-4")
            .child(issue_row(
                "SIG-12",
                "Park the native view in a surface hole",
            )),
    )
}

fn issue_row(id: &str, title: &str) -> impl IntoNode {
    interactive(
        container()
            .child(text(id).class("font-mono text-sm"))
            .child(text(title).class("text-sm")),
    )
    .class("flex-row items-center gap-3 p-2 bg-surface hover:bg-accent-9")
}
