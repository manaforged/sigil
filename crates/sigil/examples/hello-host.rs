#![allow(clippy::print_stdout)]

use sigil::Host;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::scene::SceneCommand;

fn main() {
    Runtime::scope("hello-host", || {
        let count = signal(0_i32);
        let label_text = derived(move || format!("Count: {}", count.get()));
        let mut host = Host::new();
        host.mount(move || {
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
                .child(surface(1))
        });
        count.set(1);
        let scene = match host.scene((480.0, 240.0)) {
            Ok(scene) => scene,
            Err(err) => {
                eprintln!("{err}");
                return;
            }
        };
        for command in &scene.commands {
            if let SceneCommand::GlyphRun(draw) = command {
                let rect = draw.run.rect;
                println!(
                    "{} @ ({:.0},{:.0}) {}x{}",
                    draw.run.text, rect.origin.x, rect.origin.y, rect.width, rect.height
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
