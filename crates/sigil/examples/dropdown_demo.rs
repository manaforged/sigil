//! Dropdown demo — Pack 3 D2 showcase.
#![allow(missing_docs)]

use sigil::prelude::*;

fn main() {
    let selected = signal(0_usize);
    let options = vec![
        "Rust".to_owned(),
        "Go".to_owned(),
        "Zig".to_owned(),
        "C++".to_owned(),
    ];

    launch(AppConfig::default(), move |cx| {
        cx.add(dropdown(selected, options.clone()).on_change(move || {
            println!("selected: {}", selected.get());
        }))
    });
}
