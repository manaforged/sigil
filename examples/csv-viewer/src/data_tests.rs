#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn generate_emits_requested_count() {
    let rows = generate(10);
    assert_eq!(rows.len(), 10);
}

#[test]
fn each_row_has_six_cells() {
    let rows = generate(3);
    for row in &rows {
        assert_eq!(row.cells.len(), HEADERS.len());
    }
}

#[test]
fn first_row_id_is_one() {
    let rows = generate(1);
    assert_eq!(rows[0].cell(0), "000001");
}

#[test]
fn ticker_rotates_through_table() {
    let rows = generate(TICKERS.len() + 1);
    assert_eq!(rows[0].cell(1), rows[TICKERS.len()].cell(1));
}

#[test]
fn out_of_range_cell_returns_empty() {
    let rows = generate(1);
    assert_eq!(rows[0].cell(99), "");
}
