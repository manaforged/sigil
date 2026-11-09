use super::*;

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
