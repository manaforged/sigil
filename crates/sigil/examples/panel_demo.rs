#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - panel demo").size(800, 600), build)
}

fn build() -> impl IntoNode {
    root()
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
                .child(text("Panel with child content").nowrap()),
        )
        .child(panel().elevation(3).title("Elevation 3 — modal"))
}
