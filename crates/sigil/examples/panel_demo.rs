//! Panel widget demo — themed surface cards at every elevation tier.

#![allow(missing_docs)]

use app as _;
use events as _;
use hal as _;
use kernel as _;
use layout as _;
use reactive as _;
use renderer as _;
use style as _;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - panel demo", 800, 600), build)
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(root())
}

fn root() -> NodeBuilder {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(16.0).padding(24.0))
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
