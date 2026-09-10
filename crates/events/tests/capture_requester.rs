#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::RefCell;
use std::rc::Rc;

use sigil_events::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_reactive::Runtime;

use routing_helpers::{compute_layout, pointer_down, pointer_move, route_event};

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Up,
    }))
}

#[test]
fn capture_binds_to_the_requesting_ancestor_not_the_hit_leaf() {
    Runtime::scope("events_capture_requester", || {
        let (mut cx, root, moves, ups) = tree();

        let layout = compute_layout(&mut cx, root, 400.0, 300.0);
        let mut router = EventRouter::new();

        route_event(&mut router, &mut cx, &layout, pointer_down(10.0, 10.0));
        route_event(&mut router, &mut cx, &layout, pointer_move(200.0, 150.0));
        route_event(&mut router, &mut cx, &layout, pointer_move(350.0, 250.0));
        route_event(&mut router, &mut cx, &layout, pointer_up(350.0, 250.0));

        assert_eq!(
            moves.borrow().as_slice(),
            &[(200.0, 150.0), (350.0, 250.0)],
            "captured moves dispatch to the requesting parent, not the leaf",
        );
        assert_eq!(
            ups.borrow().as_slice(),
            &[(350.0, 250.0)],
            "captured up dispatches to the requesting parent",
        );

        route_event(&mut router, &mut cx, &layout, pointer_move(20.0, 20.0));
        assert_eq!(moves.borrow().len(), 2, "capture released after up");
    });
}

type Log = Rc<RefCell<Vec<(f32, f32)>>>;

fn tree() -> (Cx, NodeId, Log, Log) {
    let moves: Log = Rc::new(RefCell::new(Vec::new()));
    let ups: Log = Rc::new(RefCell::new(Vec::new()));
    let record_move = {
        let moves = Rc::clone(&moves);
        move |ev: &mut PointerEvent| {
            if ev.is_captured() {
                let at = ev.window_position;
                moves.borrow_mut().push((at.x, at.y));
            }
        }
    };
    let record_up = {
        let ups = Rc::clone(&ups);
        move |ev: &mut PointerEvent| {
            if ev.is_captured() {
                let at = ev.window_position;
                ups.borrow_mut().push((at.x, at.y));
            }
        }
    };
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(400.0, 300.0))
            .child(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(100.0, 60.0))
                    .pointer_down(|ev| {
                        ev.capture_pointer();
                    })
                    .pointer_move(record_move)
                    .pointer_up(record_up)
                    .child(container().layout(LayoutStyle::flex_column().fixed_size(50.0, 30.0))),
            ),
    );
    (cx, root, moves, ups)
}
