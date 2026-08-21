#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Dropdown demo — Pack 3 D2 showcase.
#![allow(missing_docs, clippy::print_stdout)]

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
    cx.add(dropdown(selected, options).change(move |index| {
        println!("selected index: {index}");
    }))
}
