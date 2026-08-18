//! Slider widget demo — horizontal and vertical orientations.

#![allow(missing_docs)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - slider demo", 400, 300), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let h_val = signal(50.0_f32);
    let v_val = signal(25.0_f32);
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(24.0).padding(32.0))
            .class(classes!("w-full h-full bg-surface"))
            .child(slider(h_val, 0.0..=100.0).change(move |v| h_val.set(v)))
            .child(
                slider(v_val, 0.0..=100.0)
                    .orientation(Orientation::Vertical)
                    .change(move |v| v_val.set(v)),
            ),
    )
}
