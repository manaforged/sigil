//! Progress widget demo — determinate and indeterminate bars.

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
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - progress demo", 800, 400), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let det = signal(60.0_f32);
    let phase = signal(0.0_f32);
    effect(move || {
        let next = (phase.get() + 0.005) % 1.0;
        phase.set(next);
    });
    cx.add(root(det, phase))
}

fn root(det: Signal<f32>, phase: Signal<f32>) -> NodeBuilder {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(24.0).padding(32.0))
        .class(classes!("w-full h-full bg-surface"))
        .child(
            panel()
                .title("Determinate (60 / 100)")
                .child(progress(det, 100.0)),
        )
        .child(
            panel()
                .title("Indeterminate")
                .child(progress(phase, 1.0).indeterminate()),
        )
}
