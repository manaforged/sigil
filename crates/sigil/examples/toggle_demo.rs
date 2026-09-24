#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - toggle demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let on = signal(false);
    let disabled = signal(true);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_row().gap(24.0).padding(32.0))
            .class(classes!("w-full h-full bg-surface"))
            .child(toggle(on).change(move |v| on.set(v)))
            .child(toggle(signal(true)))
            .child(toggle(signal(false)).disabled(disabled)),
    )
}
