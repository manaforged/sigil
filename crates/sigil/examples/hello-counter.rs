use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().gap(16.0))
            .class("p-32 bg-surface")
            .child(text(move || format!("Count: {}", count.get())))
            .child(
                button("Increment")
                    .primary()
                    .activate(move || count.update(|n| *n += 1)),
            ),
    )
}
