#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "events_support/mock_shell.rs"]
mod mock_shell;
#[path = "events_support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::RefCell;
use std::rc::Rc;

use sigil::events::prelude::*;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

use routing_helpers::{compute_layout, focus_tree, pointer_down, record_focus, route_event, tab};

#[test]
fn pointer_down_focuses_nearest_focusable_ancestor() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(80.0, 40.0))
                    .child(container().layout(LayoutStyle::block().fixed_size(60.0, 20.0))),
            ),
    );
    let field = cx.nth_child(root, 0).unwrap();
    let inner = cx.nth_child(field, 0).unwrap();
    assert!(
        !cx.get(inner).unwrap().a11y.focusable,
        "inner leaf is not focusable"
    );
    let layout = compute_layout(&mut cx, root, 100.0, 100.0);
    let mut router = EventRouter::new();

    route_event(&mut router, &mut cx, &layout, pointer_down(10.0, 10.0));

    assert_eq!(
        router.focus(),
        Some(field),
        "pointer-down on a non-focusable child focuses the nearest focusable ancestor",
    );
}

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
                    .focus_in(record_focus(&order, "a-in"))
                    .focus_out(record_focus(&order, "a-out")),
            )
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(10.0, 10.0))
                    .focus_in(record_focus(&order, "b-in")),
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
