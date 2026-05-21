//! Text field widget demo — empty, placeholder, and pre-filled.

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
    sigil::launch(("Sigil - text_field demo", 400, 300), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let name = signal(String::new());
    let url = signal(String::from("https://example.com"));
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
            .class("w-full h-full bg-surface")
            .child(
                text_field(name)
                    .placeholder("Enter your name")
                    .change(move |v| {
                        name.update(|s| {
                            s.clear();
                            s.push_str(v);
                        })
                    }),
            )
            .child(text_field(url).submit(|v| println!("submit: {v}"))),
    )
}
