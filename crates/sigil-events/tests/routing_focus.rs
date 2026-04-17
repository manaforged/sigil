#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::RefCell;
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

use routing_helpers::{compute_layout, focus_tree, record_focus, route_event, tab};

#[test]
fn tab_cycles_in_insertion_order() {
    let (mut cx, root, ids) = focus_tree([None, None, None]);
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(ids[0]));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(ids[1]));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(ids[2]));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(ids[0]));
    route_event(&mut router, &mut cx, &layout, tab(true));
    assert_eq!(router.focus(), Some(ids[2]));
}

#[test]
fn tab_index_overrides_insertion_order() {
    let (mut cx, root, ids) = focus_tree([None, Some(1), None]);
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, tab(false));

    assert_eq!(router.focus(), Some(ids[1]));
}

#[test]
fn focus_in_out_fires_on_both_ends() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                    .on_focus_in(record_focus(&order, "a-in"))
                    .on_focus_out(record_focus(&order, "a-out")),
            )
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                    .on_focus_in(record_focus(&order, "b-in")),
            ),
    );
    let first = cx.nth_child(root, 0).unwrap();
    let second = cx.nth_child(root, 1).unwrap();
    let _layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    router.set_focus(&mut cx, Some(first));
    router.set_focus(&mut cx, Some(second));

    assert_eq!(&*order.borrow(), &["a-in", "a-out", "b-in"]);
}

#[test]
fn a11y_focus_update_reflects_focus_change() {
    let (mut cx, root, ids) = focus_tree([None, None, None]);
    let _layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    router.set_focus(&mut cx, Some(ids[1]));
    let update = router.a11y_focus_update();

    assert_eq!(update.focus, ids[1].to_accesskit());
}

#[test]
fn tab_ring_rebuilds_after_focusability_mutation() {
    let (mut cx, root, ids) = focus_tree([None, None, None]);
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(ids[0]));
    cx.get_mut(ids[1]).unwrap().a11y.focusable = false;
    cx.mark_mutated();
    route_event(&mut router, &mut cx, &layout, tab(false));

    assert_eq!(router.focus(), Some(ids[2]));
}
