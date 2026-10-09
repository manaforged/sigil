#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{ChipBuilder, GaugeBuilder, ProgressBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - indicators").size(480, 360), build)
}

pub(crate) fn chip_demo() -> ChipBuilder {
    chip("Stable", Tone::Success)
}

pub(crate) fn dot_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(dot(Tone::Success))
        .child(dot(Tone::Warn))
        .child(dot(Tone::Error))
}

pub(crate) fn gauge_demo() -> GaugeBuilder {
    gauge(signal(40.0)).max(100.0).label("Battery").warn(0.25)
}

pub(crate) fn progress_demo() -> ProgressBuilder {
    progress(signal(60.0), 100.0)
}

pub(crate) fn spinner_demo() -> NodeBuilder {
    spinner()
}

fn build() -> impl IntoNode {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
        .child(
            container()
                .layout(LayoutStyle::flex_row().gap(cols(1.0)))
                .child(chip_demo())
                .child(dot_demo())
                .child(spinner_demo()),
        )
        .child(gauge_demo())
        .child(progress_demo())
}
