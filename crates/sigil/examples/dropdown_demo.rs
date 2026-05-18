//! Dropdown demo — Pack 3 D2 showcase.
#![allow(missing_docs)]

use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;

use sigil::prelude::*;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - dropdown demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let selected = signal(0_usize);
    let options = vec![
        "Rust".to_owned(),
        "Go".to_owned(),
        "Zig".to_owned(),
        "C++".to_owned(),
    ];
    cx.add(dropdown(selected, options).on_change(move || {
        println!("selected: {}", selected.get());
    }))
}
