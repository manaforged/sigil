#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Checkbox widget demo — standalone and labelled variants.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - checkbox demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let checked = signal(false);
    let disabled = signal(true);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
            .class(classes!("w-full h-full bg-surface"))
            .child(checkbox(checked).change(move |v| checked.set(v)))
            // Contrast shot: neutral stays gray while the default is accent.
            .child(checkbox(signal(true)).tone(Tone::Neutral))
            .child(checkbox(signal(false)).disabled(disabled)),
    )
}
