//! Filter + sort derivation for the headless table core (DR-057 D1).

use std::cmp::Ordering;
use std::rc::Rc;

use super::columns::TableColumn;
use super::state::{SortDir, SortKey};

/// Filter-then-sort the row indices. Quick filter keeps a row when ANY
/// filterable column matches; the sort is stable so equal keys keep
/// source order (spreadsheet expectations).
pub(super) fn compute_visible<Row>(
    rows: &Rc<Vec<Row>>,
    columns: &[TableColumn<Row>],
    sort: Option<SortKey>,
    quick_filter: &str,
) -> Vec<u32> {
    let mut indices = filtered_indices(rows, columns, quick_filter);
    if let Some(key) = sort
        && let Some(column) = columns.iter().find(|col| col.id() == key.column)
        && let Some(compare) = column.compare()
    {
        indices.sort_by(|&a, &b| {
            let (Some(row_a), Some(row_b)) = (rows.get(a as usize), rows.get(b as usize)) else {
                return Ordering::Equal;
            };
            let ordering = compare(row_a, row_b);
            match key.dir {
                SortDir::Ascending => ordering,
                SortDir::Descending => ordering.reverse(),
            }
        });
    }
    indices
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "row counts are bounded far below u32::MAX"
)]
fn filtered_indices<Row>(
    rows: &Rc<Vec<Row>>,
    columns: &[TableColumn<Row>],
    quick_filter: &str,
) -> Vec<u32> {
    if quick_filter.is_empty() {
        return (0..rows.len() as u32).collect();
    }
    let filters = columns
        .iter()
        .filter_map(TableColumn::filter)
        .collect::<Vec<_>>();
    if filters.is_empty() {
        return (0..rows.len() as u32).collect();
    }
    rows.iter()
        .enumerate()
        .filter(|(_, row)| filters.iter().any(|matches| matches(row, quick_filter)))
        .map(|(index, _)| index as u32)
        .collect()
}
