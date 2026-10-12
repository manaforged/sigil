#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use super::*;

fn fixed(scroll_y: f32, rows: usize, overscan: usize) -> VisibleWindow {
    visible_window(scroll_y, RowHeights::fixed(rows, 32.0), 480.0, overscan)
}

const fn window(first: usize, end: usize) -> VisibleWindow {
    VisibleWindow { first, end }
}

#[test]
fn fixed_height_windows() {
    assert_eq!(fixed(0.0, 100, 1), window(0, 30));
    assert_eq!(fixed(1.0, 100, 1), window(0, 31));
    assert_eq!(fixed(0.0, 100, 0), window(0, 15));
    assert_eq!(fixed(1_600.0, 100, 0), window(50, 65));
    assert_eq!(fixed(1_600.0, 100, 1), window(35, 80));
    assert_eq!(fixed(99_999.0, 100, 1), window(85, 100));
    assert_eq!(fixed(0.0, 0, 1), window(0, 0));
}

#[test]
fn one_page_jump_stays_inside_the_mounted_window() {
    let here = fixed(0.0, 1_000, 1);
    let next = fixed(480.0, 1_000, 1);
    assert!(next.first < here.end);
    assert!(here.first < next.end);
}

#[test]
fn variable_height_prefix_sum_locates_rows() {
    let tops = [0.0, 30.0, 80.0, 100.0, 140.0, 200.0];
    let heights = RowHeights::Tops(&tops);
    for (actual, expected) in [
        (heights.top(0), 0.0),
        (heights.top(2), 80.0),
        (heights.top(4), 140.0),
        (heights.total(), 200.0),
    ] {
        assert!((actual - expected).abs() <= f32::EPSILON);
    }
    assert_eq!(heights.row_at(85.0), 2);
    assert_eq!(heights.row_at(199.0), 4);
    assert_eq!(heights.row_at(0.0), 0);
    assert_eq!(visible_window(85.0, heights, 100.0, 1), window(0, 5));
}

#[test]
#[allow(clippy::cast_precision_loss, reason = "small test indices")]
fn variable_height_pads_one_viewport_per_overscan() {
    let tops: Vec<f32> = (0..21).map(|i| i as f32 * 20.0).collect();
    assert_eq!(
        visible_window(80.0, RowHeights::Tops(&tops), 40.0, 1),
        window(2, 8)
    );
    assert_eq!(
        visible_window(80.0, RowHeights::Tops(&tops), 40.0, 0),
        window(4, 6)
    );
}
