//! Dropdown demo — Pack 3 D2 showcase.
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
    cx.add(dropdown(selected, options).change(move || {
        println!("selected: {}", selected.get());
    }))
}
