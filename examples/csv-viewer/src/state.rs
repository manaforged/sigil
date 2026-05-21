//! Reactive state for the CSV viewer.

use sigil::prelude::{Signal, signal};

use crate::data::{Row, generate};

use super::data::ROW_COUNT;

/// Shared state across the CSV viewer.
///
/// All fields are either `Signal<_>` (Copy) or `&'static`, so the whole
/// struct is `Copy` — every helper passes it by value without tripping
/// `needless_pass_by_value`.
#[derive(Clone, Copy)]
pub(crate) struct AppState {
    /// Parsed rows (immutable for this read-only viewer).
    pub(crate) rows: &'static [Row],
    /// Currently-selected row index, or `None`.
    pub(crate) selected: Signal<Option<usize>>,
}

impl AppState {
    /// Build state with the synthesised default corpus.
    pub(crate) fn new() -> Self {
        let rows: &'static [Row] = Box::leak(generate(ROW_COUNT).into_boxed_slice());
        Self {
            rows,
            selected: signal(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil::testing::with_cx;

    #[test]
    fn state_starts_with_full_corpus_and_no_selection() {
        with_cx("csv-viewer-state-init", |_cx| {
            let state = AppState::new();
            assert_eq!(state.rows.len(), ROW_COUNT);
            assert_eq!(state.selected.peek(), None);
        });
    }

    #[test]
    fn selecting_row_persists_in_signal() {
        with_cx("csv-viewer-state-select", |_cx| {
            let state = AppState::new();
            state.selected.set(Some(42));
            assert_eq!(state.selected.peek(), Some(42));
        });
    }
}
