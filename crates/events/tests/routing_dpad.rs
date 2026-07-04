//! Directional (dpad / stick-shaped) focus navigation — spatial
//! resolution against laid-out rects, independent of Tab order.

#![allow(missing_docs)]
// Integration tests intentionally keep assertions direct and use
// indexing for fixed test topology. See the hygiene framework.
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use accesskit as _;
use allocation_counter as _;
use criterion as _;
use reactive as _;
use smallvec as _;
use style as _;

use events::prelude::*;
use kernel::prelude::*;
use layout::prelude::*;

use routing_helpers::{abs_rect, compute_layout};

fn focusable_at(x: f32, y: f32, w: f32, h: f32) -> NodeBuilder {
    container().focusable(true).layout(abs_rect(x, y, w, h))
}

#[test]
fn dpad_right_moves_to_nearest_in_row() {
    // Three focusables in a row: left-most, a near middle, and a far
    // right one. Right must land on the middle — nearest in-direction,
    // not simply "any candidate to the right".
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(500.0, 100.0))
            .child(focusable_at(0.0, 0.0, 80.0, 20.0))
            .child(focusable_at(100.0, 0.0, 80.0, 20.0))
            .child(focusable_at(300.0, 0.0, 80.0, 20.0)),
    );
    let left = cx.nth_child(root, 0).unwrap();
    let middle = cx.nth_child(root, 1).unwrap();
    let layout = compute_layout(&mut cx, root, 500.0, 100.0);
    let mut router = EventRouter::new();

    router.set_focus(&mut cx, Some(left));
    router.focus_intent(&mut cx, &layout, FocusIntent::Right);

    assert_eq!(router.focus(), Some(middle));
}

#[test]
fn dpad_up_from_bottom_grid_row() {
    // A 2x2 grid. From bottom-right, Up must land on top-right (same
    // column, zero cross-axis deviation) rather than top-left (same
    // travel distance but a much larger cross-axis deviation).
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(300.0, 300.0))
            .child(focusable_at(0.0, 0.0, 80.0, 80.0))
            .child(focusable_at(100.0, 0.0, 80.0, 80.0))
            .child(focusable_at(0.0, 100.0, 80.0, 80.0))
            .child(focusable_at(100.0, 100.0, 80.0, 80.0)),
    );
    let top_right = cx.nth_child(root, 1).unwrap();
    let bottom_right = cx.nth_child(root, 3).unwrap();
    let layout = compute_layout(&mut cx, root, 300.0, 300.0);
    let mut router = EventRouter::new();

    router.set_focus(&mut cx, Some(bottom_right));
    router.focus_intent(&mut cx, &layout, FocusIntent::Up);

    assert_eq!(router.focus(), Some(top_right));
}

#[test]
fn dpad_no_candidate_falls_back_tab() {
    // Focus the right-most node in a row of three; there is no
    // candidate further right, so Right must fall back to Tab-order
    // next, which wraps to the first (left-most) node.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(300.0, 100.0))
            .child(focusable_at(0.0, 0.0, 80.0, 20.0))
            .child(focusable_at(100.0, 0.0, 80.0, 20.0))
            .child(focusable_at(200.0, 0.0, 80.0, 20.0)),
    );
    let left = cx.nth_child(root, 0).unwrap();
    let right = cx.nth_child(root, 2).unwrap();
    let layout = compute_layout(&mut cx, root, 300.0, 100.0);
    let mut router = EventRouter::new();

    router.set_focus(&mut cx, Some(right));
    router.focus_intent(&mut cx, &layout, FocusIntent::Right);

    assert_eq!(router.focus(), Some(left));
}
