//! Text field widget demo — empty, placeholder, and pre-filled.

#![allow(missing_docs, clippy::print_stdout)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - text_field demo", 400, 300), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let name = signal(String::new());
    let url = signal(String::from("https://example.com"));
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
            .class(classes!("w-full h-full bg-surface"))
            .child(
                text_field(name)
                    .placeholder("Enter your name")
                    .change(move |v| {
                        name.update(|s| {
                            s.clear();
                            s.push_str(v);
                        });
                    }),
            )
            .child(text_field(url).submit(|v| println!("submit: {v}"))),
    )
}
