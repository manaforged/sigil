#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - progress demo").size(800, 400),
        build,
    )
}

fn build() -> impl IntoNode {
    let det = signal(60.0_f32);
    let phase = signal(0.0_f32);
    effect(move || {
        let next = (phase.get() + 0.005) % 1.0;
        phase.set(next);
    });
    root(det, phase)
}

fn root(det: Signal<f32>, phase: Signal<f32>) -> NodeBuilder {
    column()
        .role(Role::Main)
        .fill()
        .gap(Space::Xl)
        .padding(Space::Xxl)
        .background(Token::Surface)
        .child(card(progress(det).max(100.0)).title("Determinate (60 / 100)"))
        .child(card(progress(phase).indeterminate()).title("Indeterminate"))
}
