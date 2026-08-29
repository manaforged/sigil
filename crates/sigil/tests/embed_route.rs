#![allow(missing_docs, unused_crate_dependencies, clippy::unwrap_used)]

//! `Host::route` and `Host::painted` through the public facade.
//! Layout, not `scene`: this path does not need feature `render`.

use std::cell::Cell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};

const fn pointer(x: f32, y: f32, phase: PointerPhase, button: Option<MouseButton>) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[test]
fn routed_click_reaches_button_activate() {
    Runtime::scope("embed_route", || {
        let clicked = Rc::new(Cell::new(false));
        let seen = clicked.clone();

        let mut ui = Host::new();
        ui.build(|cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(200.0, 100.0))
                    .child(button("hit me").activate(move || seen.set(true))),
            )
        });
        ui.layout((200.0, 100.0)).expect("layout");

        let mut router = EventRouter::new();
        ui.route(
            &mut router,
            pointer(20.0, 20.0, PointerPhase::Down, Some(MouseButton::Left)),
        );
        ui.route(
            &mut router,
            pointer(20.0, 20.0, PointerPhase::Up, Some(MouseButton::Left)),
        );

        assert!(
            clicked.get(),
            "routed click must reach the button's activate handler"
        );
    });
}

#[test]
fn painted_and_surfaces_use_prelude_types() {
    Runtime::scope("embed_painted", || {
        let mut host = Host::new();
        host.build(|cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(80.0, 24.0))
                    .child(text("hello"))
                    .child(surface(7)),
            )
        });
        host.layout((80.0, 24.0)).expect("layout");
        let painted: Vec<Painted> = host.painted();
        assert!(
            painted
                .iter()
                .any(|node| node.text.as_deref() == Some("hello")),
            "expected hello in {painted:?}"
        );
        let holes = host.surfaces();
        let hole = holes.first().copied().expect("one surface hole");
        assert_eq!(hole.0, 7);
        assert!(hole.1.width > 0.0);
    });
}
