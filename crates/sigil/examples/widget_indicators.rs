#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - indicators").size(480, 360), build)
}

pub(crate) fn chip_demo() -> impl IntoNode {
    chip("Stable", Tone::Success)
}

pub(crate) fn dot_demo() -> NodeBuilder {
    row()
        .gap(Space::Sm)
        .align_items(AlignItems::Stretch)
        .child(dot(Tone::Success))
        .child(dot(Tone::Warning))
        .child(dot(Tone::Error))
}

pub(crate) fn gauge_demo() -> impl IntoNode {
    gauge(signal(40.0)).max(100.0).label("Battery").warn(0.25)
}

pub(crate) fn progress_demo() -> impl IntoNode {
    progress(signal(60.0), 100.0)
}

pub(crate) fn spinner_demo() -> NodeBuilder {
    spinner()
}

fn build() -> impl IntoNode {
    column()
        .role(Role::Main)
        .gap(rows(1.0))
        .padding(cols(2.0))
        .child(
            row()
                .gap(cols(1.0))
                .align_items(AlignItems::Stretch)
                .child(chip_demo())
                .child(dot_demo())
                .child(spinner_demo()),
        )
        .child(gauge_demo())
        .child(progress_demo())
}
