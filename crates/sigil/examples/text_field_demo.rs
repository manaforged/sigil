//! Text field widget demo — empty, placeholder, and pre-filled.

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
    sigil::launch(("Sigil - text_field demo", 400, 300), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let name = signal(String::new());
    let url = signal(String::from("https://example.com"));
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap_px(16.0).padding_px(32.0))
            .class("w-full h-full bg-surface")
            .child(
                text_field(name)
                    .placeholder("Enter your name")
                    .on_change(move |v| {
                        name.update(|s| {
                            s.clear();
                            s.push_str(v);
                        })
                    }),
            )
            .child(text_field(url).on_submit(|v| println!("submit: {v}"))),
    )
}
