#![allow(clippy::print_stdout)]

use sigil::Host;
use sigil::prelude::*;
use sigil::reactive::Owner;

fn main() {
    Owner::scope("hello-host", || {
        let count = signal(0_i32);
        let label_text = memo(move || format!("Count: {}", count.get()));
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            column()
                .width(480.0)
                .height(240.0)
                .gap(Space::Lg)
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
        for draw in scene.glyph_runs() {
            let rect = draw.rect;
            println!(
                "{} @ ({:.0},{:.0}) {}x{}",
                draw.text(),
                rect.origin.x,
                rect.origin.y,
                rect.width,
                rect.height
            );
        }
        for (key, rect) in host.surfaces() {
            println!(
                "surface {key} @ ({:.0},{:.0}) {}x{}",
                rect.origin.x, rect.origin.y, rect.width, rect.height
            );
        }
    });
}
