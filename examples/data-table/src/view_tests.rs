use super::*;

#[test]
fn visible_range_at_top_covers_first_window() {
    let range = visible_range(0.0, TOTAL_ITEMS);
    assert_eq!(range.start, 0);
    // 38 in view + 38 overscan below.
    assert_eq!(range.end, 76);
}

#[test]
fn visible_range_empty_corpus_is_empty() {
    assert_eq!(visible_range(0.0, 0), 0..0);
}
