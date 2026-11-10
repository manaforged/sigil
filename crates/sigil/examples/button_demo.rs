#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - button demo").size(400, 200), build)
}

fn build() -> impl IntoNode {
    let count = signal(0_u32);
    row()
        .role(Role::Main)
        .fill()
        .gap(Space::Lg)
        .padding(Space::Xxl)
        .align_items(AlignItems::Stretch)
        .background(Token::Surface)
        .child(button("Neutral").activate(move || count.set(count.peek() + 1)))
        .child(button("Primary").primary().activate(|| {}))
}
