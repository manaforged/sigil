//! Scroll widget demo — tall column of buttons inside a clipped viewport.

#![allow(missing_docs)]

use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;

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
        .layout(LayoutStyle::flex_column().padding(24.0))
        .class("w-full h-full bg-surface")
        .child(scroll(items, 600.0).height(300.0))
}

fn tall_column() -> NodeBuilder {
    let mut col = container().layout(LayoutStyle::flex_column().gap(8.0));
    for i in 1..=15 {
        col = col.child(button(format!("Item {i}")).activate(|| {}));
    }
    col
}
