#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::Cell;
use std::rc::Rc;

use accesskit as _;
use allocation_counter as _;
use criterion as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

use sigil_core::prelude::*;
use sigil_events::prelude::*;
use sigil_hal::{Key, KeyPhase, LogicalKey, PhysicalKey, Shortcut};
use sigil_layout::prelude::*;

use routing_helpers::{compute_layout, ctrl_s, key_event, route_event};

#[test]
fn shortcut_dispatch_invokes_action() {
    let saves = Rc::new(Cell::new(0_u32));
    let shortcut = Shortcut::ctrl(LogicalKey::character("s"));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .keyboard_shortcut(shortcut.clone())
            .on_key(shortcut, {
                let saves = Rc::clone(&saves);
                move |_| saves.set(saves.get() + 1)
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, ctrl_s());

    assert_eq!(saves.get(), 1);
}

#[test]
fn focused_key_up_does_not_invoke_action() {
    let hits = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_key(Key::Enter, {
                let hits = Rc::clone(&hits);
                move |_| hits.set(hits.get() + 1)
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));

    route_event(
        &mut router,
        &mut cx,
        &layout,
        key_event(Key::Enter, PhysicalKey::Enter, KeyPhase::Up),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        key_event(Key::Enter, PhysicalKey::Enter, KeyPhase::Down),
    );

    assert_eq!(hits.get(), 1);
}
