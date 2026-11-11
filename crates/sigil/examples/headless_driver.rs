#![allow(clippy::print_stdout)]
#![allow(clippy::cast_possible_truncation)]

use sigil::Host;
use sigil::events::Event;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{CommandScene, GlyphRun};

const fn pointer(phase: PointerPhase, x: f32, y: f32) -> Event {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        PointerPhase::Up => (Some(MouseButton::Left), MouseButtons::empty()),
        _ => (None, MouseButtons::empty()),
    };
    Event::pointer(phase, Point::new(x, y))
        .button(button)
        .buttons(buttons)
}

fn click(host: &mut Host, name: &str) {
    let tree = host.accessibility_tree();
    let click_target = tree.nodes.iter().find_map(|(_, node)| {
        (node.label().is_some_and(|label| label == name)
            && node.supports_action(accesskit::Action::Click))
        .then(|| node.bounds())
        .flatten()
    });
    let Some(bounds) = click_target else {
        eprintln!("no clickable node named \"{name}\"");
        return;
    };
    let x = f64::midpoint(bounds.x0, bounds.x1) as f32;
    let y = f64::midpoint(bounds.y0, bounds.y1) as f32;
    println!("click \"{name}\" at ({x:.0},{y:.0})");
    host.route(pointer(PointerPhase::Move, x, y));
    host.route(pointer(PointerPhase::Down, x, y));
    host.route(pointer(PointerPhase::Up, x, y));
}

fn main() {
    Owner::scope("headless-driver", || {
        let count = signal(0_i32);
        let label_text = derived(move || format!("Count: {}", count.get()));
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            column()
                .width(480.0)
                .height(240.0)
                .gap(Space::Lg)
                .child(text(label_text))
                .child(button("Increment").activate(move || {
                    count.set(count.peek() + 1);
                }))
        });
        let viewport = (480.0, 240.0);
        if let Err(err) = host.layout(viewport) {
            eprintln!("{err}");
            return;
        }
        println!("-- before --\n{}", host.accessibility_text());
        click(&mut host, "Increment");
        if let Err(err) = host.layout(viewport) {
            eprintln!("{err}");
            return;
        }
        println!("-- after --\n{}", host.accessibility_text());
        match host.scene(viewport) {
            Ok(scene) => println!("scene text: {:?}", scene_text(&scene)),
            Err(err) => eprintln!("{err}"),
        }
    });
}

fn scene_text(scene: &CommandScene) -> Vec<&str> {
    scene.glyph_runs().map(GlyphRun::text).collect()
}
