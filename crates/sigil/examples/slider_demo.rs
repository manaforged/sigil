#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - slider demo").size(400, 300), build)
}

fn build() -> impl IntoNode {
    let h_val = signal(50.0_f32);
    let g_val = signal(65.0_f32);
    let v_val = signal(25.0_f32);
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(24.0).padding(32.0))
        .class("w-full h-full bg-surface")
        .child(slider(h_val, 0.0..=100.0).change(move |v| h_val.set(v)))
        .child(
            slider(g_val, 0.0..=100.0)
                .glass()
                .change(move |v| g_val.set(v)),
        )
        .child(
            slider(v_val, 0.0..=100.0)
                .orientation(Orientation::Vertical)
                .change(move |v| v_val.set(v)),
        )
}
