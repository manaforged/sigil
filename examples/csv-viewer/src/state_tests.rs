#![allow(clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn state_starts_with_full_corpus_and_no_selection() {
    with_cx("csv-viewer-state-init", |_cx| {
        let state = AppState::new();
        assert_eq!(state.table.rows().peek().len(), ROW_COUNT);
        assert!(state.table.selection().peek().is_empty());
    });
}

#[test]
fn selecting_row_persists_in_engine() {
    with_cx("csv-viewer-state-select", |_cx| {
        let state = AppState::new();
        state.table.select_only(42);
        assert!(state.table.selection().peek().contains(&42));
    });
}
