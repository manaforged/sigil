//! Reactive state for the CSV viewer.

use std::cmp::Ordering;

use sigil::prelude::*;

use crate::data::{HEADERS, Row, column_width, generate};

use super::data::ROW_COUNT;

/// Shared state across the CSV viewer.
///
/// The whole viewer now hangs off one [`TableState`]: sort, selection,
/// and the row corpus live in the engine; the view is `table(state)`.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Table engine over the synthesised corpus.
    pub(crate) table: TableState<Row>,
}

impl AppState {
    /// Build state with the synthesised default corpus.
    pub(crate) fn new() -> Self {
        Self {
            table: TableState::new(generate(ROW_COUNT), columns()),
        }
    }
}

/// Column definitions: every column sorts; ids and tickers would be
/// the quick-filter targets if the viewer grew a search field.
fn columns() -> Vec<TableColumn<Row>> {
    HEADERS
        .iter()
        .enumerate()
        .map(|(index, header)| {
            TableColumn::new(header, *header)
                .sortable(move |a: &Row, b: &Row| compare_cells(index, a, b))
                .width(ColWidth::Fixed(column_width(index)))
                .cell(move |row: &Row| cell_text(row.cell(index)))
        })
        .collect()
}

/// Numeric-aware cell comparison: the id column is zero-padded so
/// lexicographic order already matches numeric; qty and price parse.
fn compare_cells(column: usize, a: &Row, b: &Row) -> Ordering {
    match column {
        3 | 4 => numeric(a.cell(column), b.cell(column)),
        _ => a.cell(column).cmp(b.cell(column)),
    }
}

fn numeric(a: &str, b: &str) -> Ordering {
    let a = a.parse::<f64>().unwrap_or(0.0);
    let b = b.parse::<f64>().unwrap_or(0.0);
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn cell_text(value: &str) -> NodeBuilder {
    container().child(label(value.to_owned()))
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
}
