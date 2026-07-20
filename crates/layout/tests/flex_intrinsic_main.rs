//! Regression tests for `container_intrinsic_main` — Q1 primary fix.
//!
//! The bug: when a flex container child is itself a non-leaf container
//! with `flex_basis: Auto` and main-axis `size: Auto`, the flex
//! algorithm's main-axis basis function returned `0` (no content-
//! intrinsic fallback existed). Result: every section in a flex column
//! got `main_size = 0` and painted at the same y. See
//! `docs/audits/2026-05-20-widget-playground-broken-render.md` section
//! 2 for the live-render reproduction.
//!
//! These tests build trees where the children of a flex container are
//! themselves containers (no fixed height/width on the children) and
//! assert that the children receive non-zero main-axis extents derived
//! from their own subtree content — the CSS flexbox §9.2 step 3
//! hypothetical-size rule for the main axis, mirroring the existing
//! cross-axis path.
#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::doc_markdown, clippy::float_cmp, clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

mod stub;
use stub::stub_engine;

use sigil_hal::{Rect, Size};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

/// Available space passed to every compute (large enough that nothing
/// hits the containing block bound).
const AVAILABLE: Size = Size::fixed(800.0, 1000.0);

/// `StubMeasure` returns `chars * 8.0` wide × `16.0` high per text.
const TEXT_H: f32 = 16.0;

fn rect_of(cx: &Cx, id: NodeId) -> Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result must exist")
        .rect()
}

fn child_rect(cx: &Cx, parent: NodeId, n: usize) -> Rect {
    let id = *cx
        .get(parent)
        .expect("parent alive")
        .children()
        .get(n)
        .expect("child index in range");
    rect_of(cx, id)
}

/// Build a flex container of the given direction with `n` child
/// containers, each containing `child_leaves` text leaves with the
/// given label. None of the children carry an explicit size.
fn build_and_compute(
    outer: LayoutStyle,
    inner: &LayoutStyle,
    leaf_labels: &[&[&'static str]],
) -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let mut parent = container().layout(outer);
    for labels in leaf_labels {
        let mut row = container().layout(inner.clone());
        for label in *labels {
            row = row.child(text(*label));
        }
        parent = parent.child(row);
    }
    let root = cx.add(parent);
    stub_engine()
        .compute(&mut cx, root, AVAILABLE)
        .expect("compute succeeds");
    (cx, root)
}

/// Root flex_column with three child containers (each a flex_row with
/// two text children). None of the children have explicit heights. The
/// pre-fix behaviour gave each child `height = 0` and stacked them all
/// at y = 0. After `container_intrinsic_main` lands, each child must
/// have a height derived from its text content (≥ TEXT_H) and the
/// y-offsets must be strictly increasing.
#[test]
fn flex_column_with_auto_height_container_children_stacks_vertically() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_column(),
        &LayoutStyle::flex_row(),
        &[&["A", "B"], &["C", "D"], &["E", "F"]],
    );

    let c0 = child_rect(&cx, root, 0);
    let c1 = child_rect(&cx, root, 1);
    let c2 = child_rect(&cx, root, 2);

    for (i, r) in [c0, c1, c2].iter().enumerate() {
        assert!(
            r.height >= TEXT_H,
            "child {i} height must be ≥ {TEXT_H} (got {})",
            r.height
        );
    }

    assert!(
        c1.origin.y >= c0.origin.y + c0.height,
        "child 1 must start at or after child 0's bottom (c0.y={}, c0.h={}, c1.y={})",
        c0.origin.y,
        c0.height,
        c1.origin.y,
    );
    assert!(
        c2.origin.y >= c1.origin.y + c1.height,
        "child 2 must start at or after child 1's bottom (c1.y={}, c1.h={}, c2.y={})",
        c1.origin.y,
        c1.height,
        c2.origin.y,
    );
}

/// Symmetric test for the row direction. Three child containers (each
/// a flex_column of two text leaves), no explicit widths. Pre-fix gave
/// each child `width = 0` and all three painted at the same x.
#[test]
fn flex_row_with_auto_width_container_children_distributes_horizontally() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_row(),
        &LayoutStyle::flex_column(),
        &[&["AA", "BB"], &["CC", "DD"], &["EE", "FF"]],
    );

    let c0 = child_rect(&cx, root, 0);
    let c1 = child_rect(&cx, root, 1);
    let c2 = child_rect(&cx, root, 2);

    // StubMeasure: 2 chars * 8.0 = 16.0 px per text leaf.
    for (i, r) in [c0, c1, c2].iter().enumerate() {
        assert!(
            r.width >= 16.0,
            "child {i} width must be ≥ 16 (got {})",
            r.width
        );
    }

    assert!(
        c1.origin.x >= c0.origin.x + c0.width,
        "child 1 must start at or after child 0's right edge (c0.x={}, c0.w={}, c1.x={})",
        c0.origin.x,
        c0.width,
        c1.origin.x,
    );
    assert!(
        c2.origin.x >= c1.origin.x + c1.width,
        "child 2 must start at or after child 1's right edge (c1.x={}, c1.w={}, c2.x={})",
        c1.origin.x,
        c1.width,
        c2.origin.x,
    );
}

/// Nested case: an outer flex_column contains an inner flex_column
/// (no fixed height) which contains three text leaves. The outer's
/// child (the inner column) must have a height ≥ the sum of inner text
/// heights — the intrinsic main-size recursion must propagate through
/// the nested container.
#[test]
fn nested_flex_column_in_flex_column_propagates_intrinsic() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_column(),
        &LayoutStyle::flex_column(),
        &[&["L1", "L2", "L3"]],
    );

    let inner = child_rect(&cx, root, 0);
    let min = TEXT_H * 3.0;
    assert!(
        inner.height >= min,
        "inner column height must be ≥ {min} (sum of inner text heights), got {}",
        inner.height,
    );
}
