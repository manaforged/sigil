#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - toggle demo").size(400, 200), build)
}

fn build() -> impl IntoNode {
    let on = signal(false);
    let disabled = signal(true);
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_row().gap(24.0).padding(32.0))
        .class("w-full h-full bg-surface")
        .child(toggle(on).change(move |v| on.set(v)))
        .child(toggle(signal(true)))
        .child(toggle(signal(false)).disabled(disabled))
}
