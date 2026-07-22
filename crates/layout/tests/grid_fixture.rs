#![allow(unused_crate_dependencies)]
//! Minimal grid solver geometry.
//!
//! Grid was a loud `GridUnsupported` reject; it now lays out explicit
//! column tracks (`Px` / `Fr` / `Auto`) with row-major auto-flow, gaps,
//! and content-sized rows. These pin the exact resolved rects: column
//! sizing (fr distribution after fixed tracks + gaps), row-major
//! placement, and auto row heights (tallest cell per row).

#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used, clippy::indexing_slicing)]

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
use sigil_layout::{LayoutResult, TrackSize};
use sigil_style::NodeStyleExt;

use stub::stub_engine;

/// `(x, y, w, h)` of a node's computed rect.
fn rect_of(cx: &Cx, id: NodeId) -> (f32, f32, f32, f32) {
    let r = cx
        .get(id)
        .and_then(|n| n.get_extension::<LayoutResult>())
        .expect("computed layout")
        .rect();
    (r.origin.x, r.origin.y, r.width, r.height)
}

/// A row-major grid of `cells` fixed-height children over `columns`,
/// laid out in a `w`-wide box. Returns the child rects in order.
fn grid_rects(
    columns: Vec<TrackSize>,
    gap: f32,
    w: f32,
    cells: &[f32],
) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(LayoutStyle::grid(columns).width(w).gap(gap));
    for &h in cells {
        // Auto width so the column width (justify: stretch) drives it.
        root_b = root_b.child(container().layout(LayoutStyle::block().height(h)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(w, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    kids.into_iter().map(|k| rect_of(&cx, k)).collect()
}

#[test]
fn two_fr_columns_split_evenly_row_major() {
    // 200 wide, two 1fr columns, no gap → 100 each. Four 30-tall cells
    // flow into a 2x2 grid.
    let rects = grid_rects(
        vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
        0.0,
        200.0,
        &[30.0; 4],
    );
    assert_eq!(rects[0], (0.0, 0.0, 100.0, 30.0));
    assert_eq!(rects[1], (100.0, 0.0, 100.0, 30.0));
    assert_eq!(rects[2], (0.0, 30.0, 100.0, 30.0));
    assert_eq!(rects[3], (100.0, 30.0, 100.0, 30.0));
}

#[test]
fn mixed_px_and_fr_columns_with_gaps() {
    // 300 wide, columns [100px, 1fr, 2fr], gap 10.
    // fixed=100, gaps=20, free=180, per-fr=60 → widths 100/60/120.
    // x offsets: 0, 110, 180.
    let rects = grid_rects(
        vec![TrackSize::Px(100.0), TrackSize::Fr(1.0), TrackSize::Fr(2.0)],
        10.0,
        300.0,
        &[30.0; 3],
    );
    assert_eq!(rects[0], (0.0, 0.0, 100.0, 30.0));
    assert_eq!(rects[1], (110.0, 0.0, 60.0, 30.0));
    assert_eq!(rects[2], (180.0, 0.0, 120.0, 30.0));
}

#[test]
fn rows_size_to_the_tallest_cell() {
    // 2 columns, gap 0. Row 0 cells 30 & 50 → row height 50; row 1
    // starts at y=50. Row 1 cells 20 & 40 → row height 40.
    let rects = grid_rects(
        vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
        0.0,
        200.0,
        &[30.0, 50.0, 20.0, 40.0],
    );
    assert_eq!(rects[0].1, 0.0, "row 0 cell y");
    assert_eq!(rects[1].1, 0.0, "row 0 cell y");
    assert_eq!(rects[2].1, 50.0, "row 1 starts below tallest row-0 cell");
    assert_eq!(rects[3].1, 50.0, "row 1 cell y");
}

/// Build a 2-column grid over `cells` (fixed heights) with explicit row
/// tracks `rows`, and return the child rects.
fn grid_rects_with_rows(rows: Vec<TrackSize>, cells: &[f32]) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(
        LayoutStyle::grid(vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)])
            .width(200.0)
            .grid_rows(rows)
            .gap(0.0),
    );
    for &h in cells {
        root_b = root_b.child(container().layout(LayoutStyle::block().height(h)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    kids.into_iter().map(|k| rect_of(&cx, k)).collect()
}

#[test]
fn explicit_px_rows_pin_row_heights() {
    // grid-template-rows [60px, 20px]; cells are 30 tall. Row 0's Px track
    // (60) overrides the measured 30, so row 1 starts at y=60.
    let rects = grid_rects_with_rows(vec![TrackSize::Px(60.0), TrackSize::Px(20.0)], &[30.0; 4]);
    assert_eq!(rects[0].1, 0.0, "row 0 top");
    assert_eq!(rects[2].1, 60.0, "row 1 starts below the 60px row-0 track");
    assert_eq!(rects[3].1, 60.0);
}

#[test]
fn mixed_px_and_auto_rows() {
    // Row 0 pinned to 50px; row 1 Auto (measured tallest = 40).
    let rects = grid_rects_with_rows(
        vec![TrackSize::Px(50.0), TrackSize::Auto],
        &[20.0, 20.0, 40.0, 40.0],
    );
    assert_eq!(rects[2].1, 50.0, "auto row 1 starts after the 50px row 0");
}

/// Build a grid over `columns` where each cell has `(height, col_span)`,
/// laid out `w` wide with no gap. Returns the child rects.
fn grid_span_rects(
    columns: Vec<TrackSize>,
    w: f32,
    cells: &[(f32, u16)],
) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(LayoutStyle::grid(columns).width(w).gap(0.0));
    for &(h, span) in cells {
        root_b =
            root_b.child(container().layout(LayoutStyle::block().height(h).grid_column_span(span)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(w, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    kids.into_iter().map(|k| rect_of(&cx, k)).collect()
}

fn grid_span_rects_dense(
    columns: Vec<TrackSize>,
    w: f32,
    cells: &[(f32, u16)],
) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(LayoutStyle::grid(columns).width(w).gap(0.0).grid_dense());
    for &(h, span) in cells {
        root_b =
            root_b.child(container().layout(LayoutStyle::block().height(h).grid_column_span(span)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(w, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    kids.into_iter().map(|k| rect_of(&cx, k)).collect()
}

#[test]
fn dense_backfills_a_gap_a_span_left_behind() {
    // 3 cols of 100. Items: span2, span2, span1. Item 0 fills row0 col0-1,
    // item 1 (span2) can't fit the lone trailing column so it wraps to
    // row1 col0-1, leaving row0 col2 empty. The span-1 item then either
    // follows the forward cursor (sparse) or backfills the row0 gap (dense).
    let cols = vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0), TrackSize::Fr(1.0)];
    let cells = &[(30.0, 2), (30.0, 2), (30.0, 1)];

    let sparse = grid_span_rects(cols.clone(), 300.0, cells);
    assert_eq!(
        sparse[2],
        (200.0, 30.0, 100.0, 30.0),
        "sparse: span-1 follows the cursor into row 1, leaving row0 col2 empty"
    );

    let dense = grid_span_rects_dense(cols, 300.0, cells);
    assert_eq!(
        dense[2],
        (200.0, 0.0, 100.0, 30.0),
        "dense: span-1 backfills the row0 col2 gap"
    );
    // The spanning items are unaffected by the flow mode.
    assert_eq!(dense[0], (0.0, 0.0, 200.0, 30.0));
    assert_eq!(dense[1], (0.0, 30.0, 200.0, 30.0));
}

#[test]
fn column_span_widens_the_item() {
    // 3 columns of 100 each (300 wide, no gap). Item 0 spans 2 → 200 wide
    // at x=0; item 1 lands in col 2 at x=200; items 2..4 wrap to row 1.
    let cols = vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0), TrackSize::Fr(1.0)];
    let rects = grid_span_rects(cols, 300.0, &[(30.0, 2), (30.0, 1), (30.0, 1), (30.0, 1)]);
    assert_eq!(rects[0], (0.0, 0.0, 200.0, 30.0), "item 0 spans 2 cols");
    assert_eq!(rects[1].0, 200.0, "item 1 in the trailing column");
    assert!(
        rects[2].1 > 0.0,
        "row full after span+1, item 2 wraps to row 1"
    );
    assert_eq!(rects[2].0, 0.0);
}

#[test]
fn span_wraps_when_it_would_overflow() {
    // 2 columns. Item 0 spans 2 → fills row 0; item 1 (span 1) can't fit
    // the remaining 0 columns, so it wraps to row 1.
    let cols = vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)];
    let rects = grid_span_rects(cols, 200.0, &[(40.0, 2), (30.0, 1)]);
    assert_eq!(
        rects[0],
        (0.0, 0.0, 200.0, 40.0),
        "span-2 fills the 2-col row"
    );
    assert_eq!(
        rects[1].1, 40.0,
        "item 1 wrapped to row 1 (below the 40-tall row 0)"
    );
    assert_eq!(rects[1].0, 0.0);
}

#[test]
fn grid_cols_class_lays_out_as_a_grid() {
    // The `grid-cols-2` utility class drives the same 2-column grid as the
    // fluent API — parser → GridColumns atom → layout adapter → solver.
    let mut cx = Cx::new_root();
    let mut root_b = container()
        .layout(LayoutStyle::block().width(200.0))
        .class("grid-cols-2");
    for _ in 0..4 {
        root_b = root_b.child(container().layout(LayoutStyle::block().height(30.0)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    let r: Vec<_> = kids.into_iter().map(|k| rect_of(&cx, k)).collect();
    assert_eq!(r[0], (0.0, 0.0, 100.0, 30.0), "class-driven col 0");
    assert_eq!(r[1].0, 100.0, "class-driven col 1");
    assert_eq!(r[2].1, 30.0, "class-driven row 1 wrap");
}

#[test]
fn minmax_columns_flex_but_honor_their_floor() {
    // Two minmax(100, 1fr) columns in 300 wide: 150 each (above the floor).
    let wide = grid_rects(
        vec![TrackSize::MinMax(100.0, 1.0), TrackSize::MinMax(100.0, 1.0)],
        0.0,
        300.0,
        &[30.0, 30.0],
    );
    assert_eq!(wide[0].2, 150.0, "1fr share is above the floor");
    assert_eq!(wide[1].2, 150.0);

    // In 150 wide, an even split (75) is below the 100 floor, so BOTH
    // freeze at their 100 min (the floor is hard) and the grid overflows
    // to 200 — exactly CSS: a 1fr track can't shrink below its min.
    let tight = grid_rects(
        vec![TrackSize::MinMax(100.0, 1.0), TrackSize::MinMax(100.0, 1.0)],
        0.0,
        150.0,
        &[30.0, 30.0],
    );
    assert_eq!(tight[0].2, 100.0, "frozen at the floor");
    assert_eq!(tight[1].2, 100.0, "also at the floor (min is hard)");
    assert_eq!(tight[1].0, 100.0, "starts after the first floored column");
}

#[test]
fn minmax_mixed_with_fixed_and_fr() {
    // [100px, minmax(80, 1fr), 1fr] in 300 wide: fixed 100, free 200 over
    // 2fr = 100 each; the minmax's 100 clears its 80 floor.
    let r = grid_rects(
        vec![
            TrackSize::Px(100.0),
            TrackSize::MinMax(80.0, 1.0),
            TrackSize::Fr(1.0),
        ],
        0.0,
        300.0,
        &[30.0; 3],
    );
    assert_eq!(r[0].2, 100.0);
    assert_eq!(r[1].2, 100.0, "minmax fr-share clears its floor");
    assert_eq!(r[2].2, 100.0);
}

/// A 2-column grid where each cell is `(height, col_span, row_span)`, no
/// gap. Returns the child rects.
fn grid_2d_rects(cells: &[(f32, u16, u16)]) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container()
        .layout(LayoutStyle::grid(vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)]).width(200.0));
    for &(h, cspan, rspan) in cells {
        root_b = root_b.child(
            container().layout(
                LayoutStyle::block()
                    .height(h)
                    .grid_column_span(cspan)
                    .grid_row_span(rspan),
            ),
        );
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    kids.into_iter().map(|k| rect_of(&cx, k)).collect()
}

#[test]
fn row_span_reserves_a_block_and_later_items_flow_around_it() {
    // 2 cols. Item 0 spans 2 rows in col 0 (occupies (0,0) and (1,0), 60
    // tall). Item 1 -> (0,1); item 2 can't use the occupied (1,0) so it
    // flows to (1,1); item 3 -> (2,0). Rows: item0 spreads 30/row over
    // rows 0-1, so row 1 starts at y=30 and row 2 at y=60.
    let r = grid_2d_rects(&[(60.0, 1, 2), (20.0, 1, 1), (25.0, 1, 1), (40.0, 1, 1)]);
    assert_eq!(r[0], (0.0, 0.0, 100.0, 60.0), "row-spanning item, col 0");
    assert_eq!(r[1], (100.0, 0.0, 100.0, 20.0), "item 1 in col 1 row 0");
    assert_eq!(
        (r[2].0, r[2].1),
        (100.0, 30.0),
        "item 2 flows AROUND the span to (row 1, col 1)"
    );
    assert_eq!(
        (r[3].0, r[3].1),
        (0.0, 60.0),
        "item 3 lands at (row 2, col 0)"
    );
}

#[test]
fn explicit_placement_reserves_and_auto_items_flow_around() {
    // 2 cols. Item 0 is explicitly placed at line (col 2, row 2) = 0-based
    // (row 1, col 1). Items 1..4 auto-flow: (0,0), (0,1), then (1,0) —
    // NOT (1,1), which the explicit item holds.
    let mut cx = Cx::new_root();
    let mut root_b = container()
        .layout(LayoutStyle::grid(vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)]).width(200.0));
    root_b = root_b.child(container().layout(LayoutStyle::block().height(30.0).grid_area(2, 2)));
    for _ in 0..3 {
        root_b = root_b.child(container().layout(LayoutStyle::block().height(30.0)));
    }
    let root = cx.add(root_b);
    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 600.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    let r: Vec<_> = kids.into_iter().map(|k| rect_of(&cx, k)).collect();
    assert_eq!(
        (r[0].0, r[0].1),
        (100.0, 30.0),
        "explicit item at (row 1, col 1)"
    );
    assert_eq!((r[1].0, r[1].1), (0.0, 0.0), "auto (0,0)");
    assert_eq!((r[2].0, r[2].1), (100.0, 0.0), "auto (0,1)");
    assert_eq!(
        (r[3].0, r[3].1),
        (0.0, 30.0),
        "auto flows to (1,0), AROUND the explicit (1,1)"
    );
}

#[test]
fn row_gap_offsets_the_second_row() {
    // 2 columns, gap 12. Row 0 is 30 tall → row 1 at y = 30 + 12 = 42.
    let rects = grid_rects(
        vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
        12.0,
        200.0,
        &[30.0; 4],
    );
    assert_eq!(rects[2].1, 42.0);
    assert_eq!(rects[3].1, 42.0);
}
