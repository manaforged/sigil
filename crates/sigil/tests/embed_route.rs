#![allow(missing_docs)]
// Integration test; direct assertions per the hygiene framework.
#![allow(clippy::panic, clippy::unwrap_used)]
// `Host` lives behind the `render` feature. A crate-level
// `#![cfg]` would strip the allows above with the rest of the file, so the
// gate lives on the items; with the feature off this crate is empty and its
// dev-deps are legitimately unused.
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

//! `Host::route` — the embedding-host event seam.
//!
//! A host-synthesized pointer click routed through `Host::route`
//! reaches a widget's activate handler.

#[cfg(feature = "render")]
use std::cell::Cell;
#[cfg(feature = "render")]
use std::rc::Rc;

#[cfg(feature = "render")]
use sigil::prelude::*;
#[cfg(feature = "render")]
use sigil::reactive::Runtime;
#[cfg(feature = "render")]
use sigil::ui::Host;
#[cfg(feature = "render")]
use sigil_events::EventRouter;
#[cfg(feature = "render")]
use sigil_hal::{
    ModifiersState, MouseButton, MouseButtons, Point, PointerEvent, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};

#[cfg(feature = "render")]
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

#[cfg(feature = "render")]
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
        // Populate the private LayoutEngine the router needs.
        let _scene = ui.scene((200.0, 100.0)).expect("layout");

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
