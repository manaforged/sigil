#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use super::*;

#[test]
fn visible_window_at_scroll_zero_returns_first_window() {
    assert_eq!(
        visible_window(0.0, 100, 32.0, 480.0, 1),
        VisibleWindow { first: 0, end: 30 }
    );
    assert_eq!(
        visible_window(1.0, 100, 32.0, 480.0, 1),
        VisibleWindow { first: 0, end: 31 }
    );
}

#[test]
fn visible_window_overscan_zero_is_geometric_page() {
    assert_eq!(
        visible_window(0.0, 100, 32.0, 480.0, 0),
        VisibleWindow { first: 0, end: 15 }
    );
    assert_eq!(
        visible_window(1_600.0, 100, 32.0, 480.0, 0),
        VisibleWindow { first: 50, end: 65 }
    );
}

#[test]
fn visible_window_mid_list_pads_one_page_each_side() {
    assert_eq!(
        visible_window(1_600.0, 100, 32.0, 480.0, 1),
        VisibleWindow { first: 35, end: 80 }
    );
}

#[test]
fn one_page_jump_stays_inside_the_mounted_window() {
    let here = visible_window(0.0, 1_000, 32.0, 480.0, 1);
    let next = visible_window(480.0, 1_000, 32.0, 480.0, 1);
    assert!(next.first < here.end);
    assert!(here.first < next.end);
}

#[test]
fn visible_window_clamps_to_data_length() {
    assert_eq!(
        visible_window(99_999.0, 100, 32.0, 480.0, 1),
        VisibleWindow {
            first: 85,
            end: 100
        }
    );
}

#[test]
fn empty_items_produces_empty_window() {
    assert_eq!(
        visible_window(0.0, 0, 32.0, 480.0, 1),
        VisibleWindow { first: 0, end: 0 }
    );
}

#[test]
fn variable_height_prefix_sum_locates_row() {
    let row_tops = variable_row_tops();

    assert_px_eq(prefix_row_top(&row_tops, 0), 0.0);
    assert_px_eq(prefix_row_top(&row_tops, 2), 80.0);
    assert_px_eq(prefix_row_top(&row_tops, 4), 140.0);
    assert_px_eq(total_height_variable(&row_tops), 200.0);
}

#[test]
fn binary_search_finds_row_at_y() {
    let row_tops = variable_row_tops();

    assert_eq!(y_to_row(&row_tops, 85.0), 2);
    assert_eq!(y_to_row(&row_tops, 199.0), 4);
    assert_eq!(y_to_row(&row_tops, 0.0), 0);
}

#[test]
fn variable_height_visible_window() {
    let row_tops = variable_row_tops();

    assert_eq!(
        visible_window_variable(85.0, &row_tops, 100.0, 1),
        VisibleWindow { first: 0, end: 5 }
    );
}

#[test]
#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn variable_height_mid_list_pads_one_viewport() {
    let mut tops = [0.0; 21];
    for (i, top) in tops.iter_mut().enumerate() {
        *top = i as f32 * 20.0;
    }
    assert_eq!(
        visible_window_variable(80.0, &tops, 40.0, 1),
        VisibleWindow { first: 2, end: 8 }
    );
}

#[test]
#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn variable_height_overscan_zero_is_geometric_page() {
    let mut tops = [0.0; 21];
    for (i, top) in tops.iter_mut().enumerate() {
        *top = i as f32 * 20.0;
    }
    assert_eq!(
        visible_window_variable(80.0, &tops, 40.0, 0),
        VisibleWindow { first: 4, end: 6 }
    );
}

fn variable_row_tops() -> [f32; 6] {
    [0.0, 30.0, 80.0, 100.0, 140.0, 200.0]
}

fn assert_px_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
}
