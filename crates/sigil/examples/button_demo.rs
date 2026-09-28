#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - button demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let count = signal(0_u32);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_row().gap(16.0).padding(32.0))
            .class("w-full h-full bg-surface")
            .child(button("Neutral").activate(move || count.set(count.peek() + 1)))
            .child(button("Primary").primary().activate(|| {})),
    )
}
