use super::rect_of;
use super::stub::stub_engine;
use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::{LayoutStyle, Space, TrackSize};
use sigil::widgets::Widget;

fn grid_span_rects(
    columns: Vec<TrackSize>,
    w: f32,
    cells: &[(f32, u16)],
) -> Vec<(f32, f32, f32, f32)> {
    let mut cx = Cx::new_root();
    let mut root_b = container().layout(LayoutStyle::grid(columns).width(w).gap(Space::None));
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
    let mut root_b = container().layout(
        LayoutStyle::grid(columns)
            .width(w)
            .gap(Space::None)
            .grid_dense(),
    );
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
