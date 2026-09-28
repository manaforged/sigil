#![allow(clippy::print_stdout)]
#![allow(clippy::cast_possible_truncation)]

use sigil::a11y::snapshot;
use sigil::prelude::*;
use sigil::scene::{CommandScene, SceneCommand};

const fn pointer(phase: PointerPhase, x: f32, y: f32) -> ShellEvent {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        PointerPhase::Up => (Some(MouseButton::Left), MouseButtons::empty()),
        _ => (None, MouseButtons::empty()),
    };
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn click(host: &mut Host, router: &mut EventRouter, name: &str) {
    let tree = host.accessibility_tree(router);
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
    host.route(router, pointer(PointerPhase::Move, x, y));
    host.route(router, pointer(PointerPhase::Down, x, y));
    host.route(router, pointer(PointerPhase::Up, x, y));
}

fn main() {
    Runtime::scope("headless-driver", || {
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
                    .child(button("Increment").activate(move || {
                        count.set(count.peek() + 1);
                    })),
            )
        });
        let mut router = EventRouter::new();
        let viewport = (480.0, 240.0);
        if let Err(err) = host.layout(viewport) {
            eprintln!("{err}");
            return;
        }
        println!(
            "-- before --\n{}",
            snapshot(&host.accessibility_tree(&router))
        );
        click(&mut host, &mut router, "Increment");
        if let Err(err) = host.layout(viewport) {
            eprintln!("{err}");
            return;
        }
        println!(
            "-- after --\n{}",
            snapshot(&host.accessibility_tree(&router))
        );
        match host.scene(viewport) {
            Ok(scene) => println!("scene text: {:?}", scene_text(&scene)),
            Err(err) => eprintln!("{err}"),
        }
    });
}

fn scene_text(scene: &CommandScene) -> Vec<&str> {
    scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::GlyphRun(draw) => Some(draw.run.text.as_str()),
            _ => None,
        })
        .collect()
}
