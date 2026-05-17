//! Progress widget demo — determinate and indeterminate bars.

#![allow(missing_docs)]

use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use taffy as _;

use sigil::prelude::*;

fn main() -> sigil::Result<()> {
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
        .layout(LayoutStyle::flex_column().gap_px(24.0).padding_px(32.0))
        .class("w-full h-full bg-surface")
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
