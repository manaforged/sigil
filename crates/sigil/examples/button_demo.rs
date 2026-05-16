//! Button widget demo — normal, primary, and danger tones.

#![allow(missing_docs)]

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
            .child(button("Normal").on_press(move || count.set(count.peek() + 1)))
            .child(button("Primary").tone(ButtonTone::Primary).on_press(|| {}))
            .child(button("Danger").tone(ButtonTone::Danger).on_press(|| {})),
    )
}
