//! Scroll widget demo — tall column of buttons inside a clipped viewport.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - scroll demo", 800, 600), build)
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(root())
}

fn root() -> NodeBuilder {
    let items = tall_column();
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().padding_px(24.0))
        .class("w-full h-full bg-surface")
        .child(scroll(items, 600.0).viewport_height(300.0))
}

fn tall_column() -> NodeBuilder {
    let mut col = container().layout(LayoutStyle::flex_column().gap_px(8.0));
    for i in 1..=15 {
        col = col.child(button(format!("Item {i}")).on_activate(|| {}));
    }
    col
}
