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

use sigil_events::prelude::*;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

use routing_helpers::{abs_rect, compute_layout};

fn focusable_at(x: f32, y: f32, w: f32, h: f32) -> NodeBuilder {
    container().focusable(true).layout(abs_rect(x, y, w, h))
}

#[test]
fn dpad_right_moves_to_nearest_in_row() {
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
