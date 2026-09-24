#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - indicators", 480, 360), build)
}

fn chip_demo() -> ChipBuilder {
    chip("Stable", Tone::Success)
}

fn badge_demo() -> ChipBuilder {
    badge("3", Tone::Error)
}

fn dot_demo() -> DotBuilder {
    dot(Tone::Warn)
}

fn gauge_demo() -> GaugeBuilder {
    let cpu = signal(0.42_f32);
    gauge(cpu).max(1.0).label("CPU").warn(0.2)
}

fn progress_demo() -> ProgressBuilder {
    let upload = signal(30.0_f32);
    progress(upload, 100.0).label("Upload")
}

fn spinner_demo() -> NodeBuilder {
    spinner()
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(
                container()
                    .layout(LayoutStyle::flex_row().gap(cols(1.0)))
                    .child(chip_demo())
                    .child(badge_demo())
                    .child(dot_demo())
                    .child(spinner_demo()),
            )
            .child(gauge_demo())
            .child(progress_demo()),
    )
}
