#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - text_field demo").size(400, 300),
        build,
    )
}

fn build() -> impl IntoNode {
    let name = signal(String::new());
    let url = signal(String::from("https://example.com"));
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
        .class("w-full h-full bg-surface")
        .child(text_field(name).placeholder("Enter your name"))
        .child(text_field(url).submit(|v| println!("submit: {v}")))
}
