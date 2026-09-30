#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::float_cmp, clippy::unwrap_used, clippy::indexing_slicing)]

#[path = "layout_stub/mod.rs"]
mod stub;

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::{LayoutResult, TrackSize, *};

use stub::stub_engine;

fn rect_of(cx: &Cx, id: NodeId) -> (f32, f32, f32, f32) {
    let r = cx
        .get(id)
        .and_then(|n| n.get_extension::<LayoutResult>())
        .expect("computed layout")
        .rect();
    (r.origin.x, r.origin.y, r.width, r.height)
}

fn grid_rects(
    columns: Vec<TrackSize>,
    gap: f32,
    w: f32,
    cells: &[f32],
) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(LayoutStyle::grid(columns).width(w).gap(gap));
    for &h in cells {
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
    let rects = grid_rects_with_rows(vec![TrackSize::Px(60.0), TrackSize::Px(20.0)], &[30.0; 4]);
    assert_eq!(rects[0].1, 0.0, "row 0 top");
    assert_eq!(rects[2].1, 60.0, "row 1 starts below the 60px row-0 track");
    assert_eq!(rects[3].1, 60.0);
}

#[test]
fn mixed_px_and_auto_rows() {
    let rects = grid_rects_with_rows(
        vec![TrackSize::Px(50.0), TrackSize::Auto],
        &[20.0, 20.0, 40.0, 40.0],
    );
    assert_eq!(rects[2].1, 50.0, "auto row 1 starts after the 50px row 0");
}

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
    assert_eq!(dense[0], (0.0, 0.0, 200.0, 30.0));
    assert_eq!(dense[1], (0.0, 30.0, 200.0, 30.0));
}

#[test]
fn column_span_widens_the_item() {
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
fn auto_cells_stretch_to_their_row() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::grid(vec![TrackSize::Fr(1.0)])
                    .width(100.0)
                    .height(200.0)
                    .grid_rows(vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)]),
            )
            .child(container().layout(LayoutStyle::block()))
            .child(container().layout(LayoutStyle::block())),
    );
    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 200.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    let r: Vec<_> = kids.into_iter().map(|k| rect_of(&cx, k)).collect();
    assert_eq!(r[0].3, 100.0, "auto cell fills row 0");
    assert_eq!(r[1].1, 100.0);
    assert_eq!(r[1].3, 100.0, "auto cell fills row 1");
}

#[test]
fn fr_rows_share_leftover_when_height_is_definite() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::grid(vec![TrackSize::Fr(1.0)])
                    .width(100.0)
                    .height(200.0)
                    .grid_rows(vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)]),
            )
            .child(container().layout(LayoutStyle::block().height(20.0)))
            .child(container().layout(LayoutStyle::block().height(20.0))),
    );
    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 200.0))
        .expect("grid computes");
    let kids = cx.get(root).unwrap().children().to_vec();
    let r: Vec<_> = kids.into_iter().map(|k| rect_of(&cx, k)).collect();
    assert_eq!(r[0].1, 0.0, "row 0 top");
    assert_eq!(r[0].3, 20.0, "cell keeps its measured height");
    assert_eq!(r[1].1, 100.0, "row 1 starts after the leftover split");
}

#[test]
fn grid_cols_class_lays_out_as_a_grid() {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(
        LayoutStyle::block()
            .width(200.0)
            .display(Display::Grid)
            .grid_columns(vec![TrackSize::Fr(1.0); 2]),
    );
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
    let wide = grid_rects(
        vec![TrackSize::MinMax(100.0, 1.0), TrackSize::MinMax(100.0, 1.0)],
        0.0,
        300.0,
        &[30.0, 30.0],
    );
    assert_eq!(wide[0].2, 150.0, "1fr share is above the floor");
    assert_eq!(wide[1].2, 150.0);

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
    let rects = grid_rects(
        vec![TrackSize::Fr(1.0), TrackSize::Fr(1.0)],
        12.0,
        200.0,
        &[30.0; 4],
    );
    assert_eq!(rects[2].1, 42.0);
    assert_eq!(rects[3].1, 42.0);
}
