//! Toggle widget demo — on/off and disabled states.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - toggle demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let on = signal(false);
    let disabled = signal(true);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_row().gap_px(24.0).padding_px(32.0))
            .class("w-full h-full bg-surface")
            .child(toggle(on).on_change(move |v| on.set(v)))
            .child(toggle(signal(true)))
            .child(toggle(signal(false)).disabled(disabled)),
    )
}
