#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - checkbox demo").size(400, 200),
        build,
    )
}

fn build() -> impl IntoNode {
    let checked = signal(false);
    let disabled = signal(true);
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
        .class("w-full h-full bg-surface")
        .child(checkbox(checked).change(move |v| checked.set(v)))
        .child(checkbox(signal(true)).tone(Tone::Neutral))
        .child(checkbox(signal(false)).disabled(disabled))
}
