#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("timer-stopwatch-smoke", |cx| {
        let root = app_tree(cx);
        let mut count = 0_usize;
        cx.visit_preorder(root, |_, _| count += 1);
        assert!(count >= 5, "expected at least 5 nodes, got {count}");
    });
}

#[test]
fn format_ms_pads_correctly() {
    assert_eq!(format_ms(0), "00:00.000");
    assert_eq!(format_ms(1_234), "00:01.234");
    assert_eq!(format_ms(61_500), "01:01.500");
}

#[test]
fn lap_list_handles_empty_and_filled() {
    with_cx("timer-stopwatch-lap-list", |cx| {
        let laps: Signal<Vec<u64>> = signal(Vec::new());
        let empty_root = cx.add(lap_list(laps));
        assert!(
            cx.first_child(empty_root).is_some(),
            "empty lap_list still renders the placeholder child"
        );

        laps.set(vec![1_500, 3_000]);
        let filled_root = cx.add(lap_list(laps));
        let mut count = 0_usize;
        cx.visit_preorder(filled_root, |_, _| count += 1);
        assert!(
            count >= 3,
            "two laps should produce at least 3 nodes, got {count}"
        );
    });
}
