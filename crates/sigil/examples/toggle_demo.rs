//! Toggle widget demo — on/off and disabled states.

#![allow(missing_docs)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - toggle demo", 400, 200), build)
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
