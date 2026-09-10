#![allow(missing_docs, clippy::print_stdout)]

use sigil::prelude::*;

fn main() {
    Runtime::scope("hello-host", || {
        let count = signal(0_i32);
        let label_text = derived(move || format!("Count: {}", count.get()));
        let mut host = Host::new();
        host.build(|cx| {
            cx.add(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fixed_size(480.0, 240.0)
                            .gap(16.0),
                    )
                    .child(text(label_text))
                    .child(button("Increment").primary().activate(move || {
                        count.set(count.peek() + 1);
                    }))
                    .child(surface(1)),
            )
        });
        count.set(1);
        if let Err(err) = host.layout((480.0, 240.0)) {
            eprintln!("{err}");
            return;
        }
        for node in host.painted() {
            if let Some(text) = &node.text {
                println!(
                    "{text} @ ({:.0},{:.0}) {}x{}",
                    node.rect.origin.x, node.rect.origin.y, node.rect.width, node.rect.height
                );
            }
        }
        for (key, rect) in host.surfaces() {
            println!(
                "surface {key} @ ({:.0},{:.0}) {}x{}",
                rect.origin.x, rect.origin.y, rect.width, rect.height
            );
        }
    });
}
