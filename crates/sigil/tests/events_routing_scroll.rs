#![cfg(feature = "kittest")]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use crate::events_support::routing_helpers;

use sigil::geometry::Point;
use sigil::widgets::Widget;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::events::*;
use sigil::geometry::Constraint;
use sigil::kernel::*;
use sigil::layout::*;

use routing_helpers::{abs_rect, compute_layout, route_event};

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

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::pointer_down(Point::new(5.0, 5.0)),
    );

    assert_eq!(hits.get(), 0);
}

#[test]
fn scroll_dispatches_via_fresh_hit_test() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let outer = Rc::clone(&order);
    let inner = Rc::clone(&order);
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .scroll(move |_| outer.borrow_mut().push("outer"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .scroll(move |_| inner.borrow_mut().push("inner")),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::wheel(Point::new(5.0, 5.0), 1.0),
    );

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
            .scroll(move |_| outer.borrow_mut().push("outer"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .scroll(move |ev| {
                        inner.borrow_mut().push("inner");
                        ev.stop_propagation();
                    }),
            ),
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(
        &mut router,
        &mut cx,
        &layout,
        Event::wheel(Point::new(5.0, 5.0), 1.0),
    );

    assert_eq!(&*order.borrow(), &["inner"]);
}

fn press(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, at: f32) {
    route_event(router, cx, layout, Event::pointer_down(Point::new(at, at)));
}

#[test]
fn pointer_hits_last_frame_boxes() {
    let old_hit = Rc::new(Cell::new(false));
    let mut cx = Cx::new_root();
    let child_hit = Rc::clone(&old_hit);
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(300.0, 300.0))
            .child(
                container()
                    .layout(abs_rect(0.0, 0.0, 80.0, 80.0))
                    .pointer_down(move |_| child_hit.set(true)),
            ),
    );
    let child = cx.nth_child(root, 0).expect("child");
    let mut layout = compute_layout(&mut cx, root, 300.0, 300.0);
    let mut router = EventRouter::new();

    press(&mut router, &mut cx, &layout, 10.0);
    assert!(old_hit.get(), "first frame box at origin is hittable");
    old_hit.set(false);

    if let Some(style) = cx
        .get_mut(child)
        .and_then(|node| node.extension_mut::<LayoutStyle>())
    {
        *style = abs_rect(200.0, 200.0, 80.0, 80.0);
    }
    cx.mark_mutated();

    press(&mut router, &mut cx, &layout, 10.0);
    assert!(old_hit.get(), "pointer still hits last frame's box");
    old_hit.set(false);
    press(&mut router, &mut cx, &layout, 210.0);
    assert!(!old_hit.get(), "this frame's intent is not hittable yet");

    layout
        .compute(&mut cx, root, Constraint::fixed(300.0, 300.0))
        .expect("recompute");
    press(&mut router, &mut cx, &layout, 210.0);
    assert!(old_hit.get(), "after compute, new box is hittable");
    old_hit.set(false);
    press(&mut router, &mut cx, &layout, 10.0);
    assert!(!old_hit.get(), "after compute, last frame's box is gone");
}
