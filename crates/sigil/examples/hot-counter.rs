use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    let terminal = std::env::args().any(|arg| arg == "--terminal");
    let config = AppConfig::new("Sigil - hot-counter").size(480, 240);
    let config = if terminal { config.terminal() } else { config };
    Runtime::scope("hot-counter", || {
        let count = signal(0_i32);
        sigil::run(config, move |cx: &mut Cx| cx.add(counter(count)))
    })
}

fn counter(count: Signal<i32>) -> NodeBuilder {
    let label = derived(move || format!("Count: {}", count.get()));
    container()
        .layout(LayoutStyle::flex_column().gap(16.0))
        .class(classes!("p-8 bg-surface"))
        .child(text(label).class(classes!("text-on-surface")))
        .child(button("Increment").primary().activate(move || {
            count.set(count.peek() + 1);
        }))
}
