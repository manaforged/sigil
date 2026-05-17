//! Button widget demo — neutral and primary tones.

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
    sigil::launch(("Sigil - button demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let count = signal(0_u32);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_row().gap_px(16.0).padding_px(32.0))
            .class("w-full h-full bg-surface")
            .child(button("Neutral").on_activate(move || count.set(count.peek() + 1)))
            .child(button("Primary").primary().on_activate(|| {})),
    )
}
