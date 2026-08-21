#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Slider widget demo — horizontal and vertical orientations.

#![allow(missing_docs)]

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
