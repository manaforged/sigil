//! W1.3 — `Display::Grid` fallback smoke fixture.
//!
//! `Grid` containers have no dedicated solver yet (post-G2 work,
//! tracked in `docs/followups.md`). Until then the engine routes grid
//! nodes through `BlockSolver`. These tests verify that:
//!
//! 1. A grid container with two block children does **not** panic.
//! 2. Children receive real computed boxes at the parent's content
//!    origin (block semantics, no margin/padding).
//! 3. Children are stacked in block order (y increases per child).

#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_reactive as _;
use slotmap as _;
use smallvec as _;
use taffy as _;

mod stub;

use sigil_hal as _;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_style as _;

use stub::stub_engine;

// ── helpers ──────────────────────────────────────────────────────────────────

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result must exist after compute")
        .rect()
}

fn children_of(cx: &Cx, parent: NodeId) -> Vec<NodeId> {
    cx.get(parent)
        .expect("parent node exists")
        .children()
        .iter()
        .copied()
        .collect()
}

// ── fixture tests ─────────────────────────────────────────────────────────────

/// Grid container + two block children: engine must not panic and both
/// children must receive valid boxes.
#[test]
fn grid_container_lays_out_without_panic() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::grid().fixed_size(200.0, 200.0))
            .child(container().layout(LayoutStyle::block().fixed_size(100.0, 40.0)))
            .child(container().layout(LayoutStyle::block().fixed_size(80.0, 30.0))),
    );

    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 200.0))
        .expect("grid container compute must succeed");

    // Root received a box.
    let root_rect = layout_rect(&cx, root);
    assert_eq!(root_rect.width, 200.0, "root width matches fixed size");
    assert_eq!(root_rect.height, 200.0, "root height matches fixed size");
}

/// Children of a grid container (routed through block semantics) must
/// be placed at the parent's content origin and stacked in block order.
#[test]
fn grid_children_receive_block_origin_boxes() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::grid().fixed_size(300.0, 300.0))
            .child(container().layout(LayoutStyle::block().fixed_size(100.0, 50.0)))
            .child(container().layout(LayoutStyle::block().fixed_size(80.0, 40.0))),
    );

    stub_engine()
        .compute(&mut cx, root, Size::fixed(300.0, 300.0))
        .expect("compute must succeed");

    let kids = children_of(&cx, root);
    let first = layout_rect(&cx, kids[0]);
    let second = layout_rect(&cx, kids[1]);

    // Both children land at x=0 (parent content origin, no padding).
    assert_eq!(first.origin.x, 0.0, "first child x at content origin");
    assert_eq!(second.origin.x, 0.0, "second child x at content origin");

    // First child anchored at y=0 (top of parent content area).
    assert_eq!(first.origin.y, 0.0, "first child y at top of content area");

    // Second child stacked below first (block semantics: y = first.h).
    assert_eq!(
        second.origin.y, first.height,
        "second child y equals first child height (block stacking)",
    );

    // Sizes are preserved.
    assert_eq!(first.width, 100.0, "first child width");
    assert_eq!(first.height, 50.0, "first child height");
    assert_eq!(second.width, 80.0, "second child width");
    assert_eq!(second.height, 40.0, "second child height");
}
