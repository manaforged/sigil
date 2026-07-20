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

use sigil_events::prelude::*;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

use routing_helpers::{
    abs_rect, compute_layout, pointer_down, pointer_move, record, route_event, scroll_lines,
    window_blur,
};

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
                    .pointer_down(move |_| bottom_hit.set(true))
            })
            .child({
                let top_hit = Rc::clone(&top_hit);
                container()
                    .layout(abs_rect(0.0, 0.0, 80.0, 80.0))
                    .pointer_down(move |_| top_hit.set(true))
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
            .pointer_down_capture(record(&order, "root"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(80.0, 80.0))
                    .pointer_down_capture(record(&order, "mid"))
                    .child(
                        container()
                            .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                            .pointer_down_capture(record(&order, "leaf")),
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
            .pointer_down_capture({
                let order = Rc::clone(&order);
                move |event| {
                    order.borrow_mut().push("root-capture");
                    event.stop_propagation();
                }
            })
            .pointer_down(record(&order, "root-bubble"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .pointer_down(record(&order, "leaf-target")),
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
                    .pointer_down(PointerEvent::capture_pointer)
                    .pointer_move({
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
fn pointer_handlers_receive_target_local_position() {
    let local = Rc::new(Cell::new((0.0_f32, 0.0_f32)));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .layout(abs_rect(40.0, 30.0, 20.0, 20.0))
                    .pointer_down({
                        let local = Rc::clone(&local);
                        move |event| local.set((event.position.x, event.position.y))
                    }),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(45.0, 37.0));

    assert_eq!(local.get(), (5.0, 7.0));
}

#[test]
fn pointer_move_synthesizes_enter_leave() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .layout(abs_rect(0.0, 0.0, 20.0, 20.0))
                    .pointer_enter(record(&order, "a-enter"))
                    .pointer_leave(record(&order, "a-leave")),
            )
            .child(
                container()
                    .layout(abs_rect(40.0, 0.0, 20.0, 20.0))
                    .pointer_enter(record(&order, "b-enter"))
                    .pointer_leave(record(&order, "b-leave")),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_move(5.0, 5.0));
    route_event(&mut router, &mut cx, &layout, pointer_move(45.0, 5.0));
    route_event(&mut router, &mut cx, &layout, pointer_move(200.0, 200.0));

    assert_eq!(
        &*order.borrow(),
        &["a-enter", "a-leave", "b-enter", "b-leave"]
    );
}

#[test]
fn window_blur_dispatches_synthetic_leaves() {
    // Regression: alt-tabbing while hovering a widget used to leave
    // hover state frozen because the router cleared `hover_path` in
    // place without firing synthetic PointerPhase::Leave events.
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .pointer_enter(record(&order, "root-enter"))
            .pointer_leave(record(&order, "root-leave"))
            .child(
                container()
                    .layout(abs_rect(0.0, 0.0, 20.0, 20.0))
                    .pointer_enter(record(&order, "child-enter"))
                    .pointer_leave(record(&order, "child-leave")),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_move(5.0, 5.0));
    assert_eq!(&*order.borrow(), &["root-enter", "child-enter"]);

    route_event(&mut router, &mut cx, &layout, window_blur());

    assert_eq!(
        &*order.borrow(),
        &["root-enter", "child-enter", "child-leave", "root-leave"]
    );
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
                    .pointer_down({
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

#[test]
fn scroll_dispatches_via_fresh_hit_test() {
    // No pointer event has fired before the first scroll — the
    // router should hit-test event.position rather than rely on a
    // stale hover path. Regression for scroll-before-first-move.
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let outer = Rc::clone(&order);
    let inner = Rc::clone(&order);
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_scroll(move |_| outer.borrow_mut().push("outer"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .on_scroll(move |_| inner.borrow_mut().push("inner")),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, scroll_lines(5.0, 5.0, 1.0));

    assert_eq!(&*order.borrow(), &["inner", "outer"]);
}

#[test]
fn scroll_stop_propagation_halts_bubble() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let outer = Rc::clone(&order);
    let inner = Rc::clone(&order);
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .on_scroll(move |_| outer.borrow_mut().push("outer"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .on_scroll(move |ev| {
                        inner.borrow_mut().push("inner");
                        ev.stop_propagation();
                    }),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, scroll_lines(5.0, 5.0, 1.0));

    assert_eq!(&*order.borrow(), &["inner"]);
}
