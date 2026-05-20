//! Checkbox widget demo — standalone and labelled variants.

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
    sigil::launch(("Sigil - checkbox demo", 400, 200), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let checked = signal(false);
    let disabled = signal(true);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(16.0).padding(32.0))
            .class("w-full h-full bg-surface")
            .child(checkbox(checked).change(move |v| checked.set(v)))
            .child(checkbox(signal(true)).tone(CheckboxTone::Primary))
            .child(checkbox(signal(false)).disabled(disabled)),
    )
}
