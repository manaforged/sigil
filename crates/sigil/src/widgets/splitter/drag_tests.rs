#![allow(clippy::indexing_slicing)]

use super::{PaneBounds, cumulative_split, nearest_snap, redistribute};

const NO_GROW: [f32; 3] = [0.0, 0.0, 0.0];
const NO_MAX: [f32; 3] = [f32::INFINITY; 3];

#[test]
fn redistribute_within_bounds() {
    let next = redistribute(
        &[200.0, 200.0, 200.0],
        0,
        50.0,
        &PaneBounds {
            mins: &[0.0, 0.0, 0.0],
            maxs: &NO_MAX,
            grows: &NO_GROW,
        },
    );
    assert_eq!(next, vec![250.0, 150.0, 200.0]);
}

#[test]
fn redistribute_clamps_to_min() {
    let next = redistribute(
        &[200.0, 200.0, 200.0],
        0,
        300.0,
        &PaneBounds {
            mins: &[0.0, 100.0, 0.0],
            maxs: &NO_MAX,
            grows: &NO_GROW,
        },
    );
    assert_eq!(next, vec![300.0, 100.0, 200.0]);
}

#[test]
fn cumulative_split_sums_through_handle() {
    assert!((cumulative_split(&[200.0, 150.0, 200.0], 0) - 200.0).abs() < f32::EPSILON);
    assert!((cumulative_split(&[200.0, 150.0, 200.0], 1) - 350.0).abs() < f32::EPSILON);
}

#[test]
fn sash_moves_only_the_fixed_pane() {
    let next = redistribute(
        &[120.0, 0.0],
        0,
        40.0,
        &PaneBounds {
            mins: &[0.0, 0.0],
            maxs: &[f32::INFINITY, f32::INFINITY],
            grows: &[0.0, 1.0],
        },
    );
    assert!(
        (next[0] - 160.0).abs() < 0.01,
        "fixed sidebar grows, got {}",
        next[0]
    );
    assert!(
        next[1].abs() < 0.01,
        "grow pane stays leftover (0 in the vector), got {}",
        next[1]
    );
}

#[test]
fn nearest_snap_within_threshold() {
    assert_eq!(nearest_snap(&[200.0], 195.0, 8.0), Some(200.0));
    assert_eq!(nearest_snap(&[200.0], 210.0, 8.0), None);
    assert_eq!(nearest_snap(&[200.0, 400.0], 395.0, 8.0), Some(400.0));
}
