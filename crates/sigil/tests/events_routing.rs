#![cfg(feature = "kittest")]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use crate::events_support::routing_helpers;

use sigil::geometry::Point;
use sigil::widgets::{Layout, Widget};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::events::*;
use sigil::kernel::*;
use sigil::layout::*;

use routing_helpers::{abs_rect, compute_layout, record, route_event};

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(10.0, 10.0)),
    );

    assert!(top_hit.get());
    assert!(!bottom_hit.get());
}

#[test]
fn file_drop_broadcasts_to_registered_handlers() {
    let dropped = Rc::new(RefCell::new(Vec::<String>::new()));
    let mut cx = Cx::new_root();
    let root = cx.add({
        let dropped = Rc::clone(&dropped);
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .file_drop(move |event: &FileDropEvent| {
                if event.phase == FileDropPhase::Dropped
                    && let Some(path) = &event.path
                {
                    dropped.borrow_mut().push(path.clone());
                }
            })
    });
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::file_drop(Some("/tmp/data.csv".to_owned()), FileDropPhase::Dropped),
    );

    assert_eq!(dropped.borrow().as_slice(), ["/tmp/data.csv"]);
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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(5.0, 5.0)),
    );

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(5.0, 5.0)),
    );

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(5.0, 5.0)),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(200.0, 200.0)),
    );

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(45.0, 37.0)),
    );

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(5.0, 5.0)),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(45.0, 5.0)),
    );
    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(200.0, 200.0)),
    );

    assert_eq!(
        &*order.borrow(),
        &["a-enter", "a-leave", "b-enter", "b-leave"]
    );
}

#[test]
fn window_blur_dispatches_synthetic_leaves() {
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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_drag(Point::new(5.0, 5.0)),
    );
    assert_eq!(&*order.borrow(), &["root-enter", "child-enter"]);

    route_event(&mut router, &mut cx, &layout, Event::blur());

    assert_eq!(
        &*order.borrow(),
        &["root-enter", "child-enter", "child-leave", "root-leave"]
    );
}
