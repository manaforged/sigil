//! Panel widget demo — themed surface cards at every elevation tier.

#![allow(missing_docs)]

use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use taffy as _;

use sigil::prelude::*;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - panel demo", 800, 600), build)
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(root())
}

fn root() -> NodeBuilder {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap_px(16.0).padding_px(24.0))
        .class("w-full h-full bg-surface")
        .child(panel().elevation(0).title("Elevation 0 — flat"))
        .child(panel().elevation(1).title("Elevation 1 — subtle"))
        .child(
            panel()
                .elevation(2)
                .title("Elevation 2 — card")
                .child(label("Panel with child content")),
        )
        .child(panel().elevation(3).title("Elevation 3 — modal"))
}
