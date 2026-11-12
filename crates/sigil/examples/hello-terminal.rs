use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - hello-terminal")
            .size(80, 24)
            .terminal(),
        app_tree,
    )
}

fn app_tree() -> impl IntoNode {
    let count = signal(0_i32);
    let label_text = memo(move || format!("Count: {}", count.get()));
    column()
        .gap(Space::raw(1.0, "example layout"))
        .child(text(label_text))
        .child(button("Increment").primary().activate(move || {
            count.set(count.peek() + 1);
        }))
}
