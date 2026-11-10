#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - scroll demo").size(800, 600), build)
}

fn build() -> impl IntoNode {
    root()
}

fn root() -> NodeBuilder {
    let items = tall_column();
    column()
        .role(Role::Main)
        .fill()
        .padding(Space::Xl)
        .gap(Space::None)
        .background(Token::Surface)
        .child(scroll(items).extent(600.0).height(300.0))
}

fn tall_column() -> NodeBuilder {
    let mut col = column().gap(Space::Sm);
    for i in 1..=15 {
        col = col.child(button(format!("Item {i}")).activate(|| {}));
    }
    col
}
