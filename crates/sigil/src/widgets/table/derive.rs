use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;

use crate::reactive::Signal;

use super::columns::TableColumn;
use super::state::{SortDir, SortKey};

pub(super) fn reuse_identity(len: usize, cache: &mut Option<Rc<Vec<u32>>>) -> Rc<Vec<u32>> {
    if let Some(existing) = cache.as_ref()
        && existing.len() == len
    {
        return Rc::clone(existing);
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "row counts are bounded far below u32::MAX"
    )]
    let fresh = Rc::new((0..len as u32).collect());
    *cache = Some(Rc::clone(&fresh));
    fresh
}

pub(super) fn compute_visible<Row>(
    rows: &[Row],
    columns: &[TableColumn<Row>],
    sort: Option<SortKey>,
    quick_filter: &str,
    previous: Option<&Rc<Vec<u32>>>,
) -> Rc<Vec<u32>> {
    let Some((key, compare)) = sort.and_then(|key| {
        columns
            .iter()
            .find(|column| column.id() == key.column)
            .and_then(TableColumn::compare)
            .map(|compare| (key, compare))
    }) else {
        return Rc::new(filtered_indices(rows, columns, quick_filter));
    };
    let order = |&a: &u32, &b: &u32| {
        let (Some(row_a), Some(row_b)) = (rows.get(a as usize), rows.get(b as usize)) else {
            return Ordering::Equal;
        };
        let ordering = compare(row_a, row_b);
        match key.dir {
            SortDir::Ascending => ordering,
            SortDir::Descending => ordering.reverse(),
        }
    };
    if quick_filter.is_empty()
        && let Some(previous) = previous
        && previous.len() == rows.len()
    {
        if previous.is_sorted_by(|a, b| order(a, b).then_with(|| a.cmp(b)).is_le()) {
            return Rc::clone(previous);
        }
        return Rc::new(reorder_indices(previous, order));
    }
    let mut indices = filtered_indices(rows, columns, quick_filter);
    indices.sort_by(order);
    Rc::new(indices)
}

#[inline(never)]
fn reorder_indices(previous: &[u32], order: impl Fn(&u32, &u32) -> Ordering) -> Vec<u32> {
    let stable = |a: &u32, b: &u32| order(a, b).then_with(|| a.cmp(b));
    let mut indices = previous.to_vec();
    let Some(inversion) = previous
        .iter()
        .zip(previous.iter().skip(1))
        .position(|(a, b)| stable(a, b).is_gt())
    else {
        return indices;
    };
    for position in [inversion, inversion + 1] {
        let moved = indices.remove(position);
        if indices
            .get(inversion.saturating_sub(1)..)
            .is_some_and(|tail| tail.is_sorted_by(|a, b| stable(a, b).is_le()))
        {
            let destination = indices.partition_point(|row| stable(row, &moved).is_lt());
            indices.insert(destination, moved);
            return indices;
        }
        indices.insert(position, moved);
    }
    for (index, value) in (0_u32..).zip(&mut indices) {
        *value = index;
    }
    indices.sort_by(order);
    indices
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "row counts are bounded far below u32::MAX"
)]
fn filtered_indices<Row>(
    rows: &[Row],
    columns: &[TableColumn<Row>],
    quick_filter: &str,
) -> Vec<u32> {
    let filters = columns
        .iter()
        .filter_map(TableColumn::filter)
        .collect::<Vec<_>>();
    if quick_filter.is_empty() || filters.is_empty() {
        return (0..rows.len() as u32).collect();
    }
    rows.iter()
        .enumerate()
        .filter(|(_, row)| filters.iter().any(|matches| matches(row, quick_filter)))
        .map(|(index, _)| index as u32)
        .collect()
}

pub(super) fn retain_visible(
    visible: &[u32],
    selection: Signal<Rc<HashSet<u32>>>,
    anchor: Signal<Option<u32>>,
) {
    let selected = selection.peek();
    if selected.is_empty() {
        return;
    }
    let shown: HashSet<u32> = visible.iter().copied().collect();
    if selected.iter().all(|row| shown.contains(row)) {
        return;
    }
    let kept: HashSet<u32> = selected
        .iter()
        .copied()
        .filter(|row| shown.contains(row))
        .collect();
    selection.set(Rc::new(kept));
    if anchor.peek().is_some_and(|row| !shown.contains(&row)) {
        anchor.set(None);
    }
}
