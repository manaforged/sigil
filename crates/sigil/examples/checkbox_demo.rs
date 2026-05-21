//! Checkbox widget demo — standalone and labelled variants.

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
