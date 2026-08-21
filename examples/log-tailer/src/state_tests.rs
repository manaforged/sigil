#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn filter_by_level_subset() {
    with_cx("log-tailer-filter-level", |_cx| {
        let state = AppState::new();
        state.level_filter.set(Some(Level::Error));
        let filtered = state.filtered();
        assert!(!filtered.is_empty());
        assert!(filtered.iter().all(|line| line.level == Level::Error));
    });
}

#[test]
fn filter_by_text_matches_message() {
    with_cx("log-tailer-filter-text", |_cx| {
        let state = AppState::new();
        state.filter_text.set(String::from("compactor"));
        let filtered = state.filtered();
        assert!(!filtered.is_empty());
        assert!(
            filtered
                .iter()
                .all(|line| line.message.contains("compactor"))
        );
    });
}

#[test]
fn empty_filter_passes_everything() {
    with_cx("log-tailer-filter-empty", |_cx| {
        let state = AppState::new();
        assert_eq!(state.filtered().len(), state.all_lines.len());
    });
}
