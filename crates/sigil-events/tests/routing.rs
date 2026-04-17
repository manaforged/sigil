#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use accesskit as _;
use allocation_counter as _;
use criterion as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

use sigil_core::prelude::*;
use sigil_events::prelude::*;
use sigil_layout::prelude::*;

use routing_helpers::{abs_rect, compute_layout, pointer_down, pointer_move, record, route_event};

#[test]
fn pointer_down_hits_top_of_z_stack() {
    let top_hit = Rc::new(Cell::new(false));
    let bottom_hit = Rc::new(Cell::new(false));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child({
                let bottom_hit = Rc::clone(&bottom_hit);
                container()
                    .layout(abs_rect(0.0, 0.0, 80.0, 80.0))
                    .on_pointer_down(move |_| bottom_hit.set(true))
            })
            .child({
                let top_hit = Rc::clone(&top_hit);
                container()
                    .layout(abs_rect(0.0, 0.0, 80.0, 80.0))
                    .on_pointer_down(move |_| top_hit.set(true))
            }),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(10.0, 10.0));

    assert!(top_hit.get());
    assert!(!bottom_hit.get());
}

#[test]
fn capture_runs_root_to_target() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_pointer_down_capture(record(&order, "root"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(80.0, 80.0))
                    .on_pointer_down_capture(record(&order, "mid"))
                    .child(
                        container()
                            .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                            .on_pointer_down_capture(record(&order, "leaf")),
                    ),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(5.0, 5.0));

    assert_eq!(&*order.borrow(), &["root", "mid", "leaf"]);
}

#[test]
fn stop_propagation_skips_bubble() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_pointer_down_capture({
                let order = Rc::clone(&order);
                move |event| {
                    order.borrow_mut().push("root-capture");
                    event.stop_propagation();
                }
            })
            .on_pointer_down(record(&order, "root-bubble"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .on_pointer_down(record(&order, "leaf-target")),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(5.0, 5.0));

    assert_eq!(&*order.borrow(), &["root-capture"]);
}

#[test]
fn pointer_capture_redirects_moves() {
    let captured_moves = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .on_pointer_down(PointerEvent::capture_pointer)
                    .on_pointer_move({
                        let captured_moves = Rc::clone(&captured_moves);
                        move |event| {
                            if event.is_captured() {
                                captured_moves.set(captured_moves.get() + 1);
                            }
                        }
                    }),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(5.0, 5.0));
    route_event(&mut router, &mut cx, &layout, pointer_move(200.0, 200.0));

    assert_eq!(captured_moves.get(), 1);
}

#[test]
fn disposed_handler_node_no_panic() {
    let hits = Rc::new(Cell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .on_pointer_down({
                        let hits = Rc::clone(&hits);
                        move |_| hits.set(hits.get() + 1)
                    }),
            ),
    );
    let child = cx.nth_child(root, 0).unwrap();
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    assert!(cx.remove(child));
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(5.0, 5.0));

    assert_eq!(hits.get(), 0);
}
