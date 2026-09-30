use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - hello-counter").size(480, 240),
        app_tree,
    )
}

fn app_tree() -> impl IntoNode {
    let count = signal(0_i32);
    column()
        .layout(LayoutStyle::default().padding(32.0))
        .bg(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(
            button("Increment")
                .primary()
                .activate(move || count.update(|n| *n += 1)),
        )
}
