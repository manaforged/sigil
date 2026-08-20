//! Row identity for [`TableState`](super::TableState).
//!
//! Selection is a backing index. When the row type implements
//! [`Identifiable`], [`super::TableState::identified`] remaps that index
//! through [`Identifiable::id`] on
//! [`replace_rows`](super::TableState::replace_rows). The invariant
//! lives on the row, not on a closure at the table.

use std::collections::HashSet;
use std::hash::Hash;

/// A stable id for one row.
///
/// Implement this on the record. [`super::TableState::identified`]
/// remaps selection.
/// [`virtual_list`](crate::virtual_list)
/// `.identified()` remaps `NodeId`. Both read [`Self::id`].
pub trait Identifiable {
    /// Stable id type. Cheap to clone and hash.
    type Id: Clone + Eq + Hash;

    /// This row's id.
    fn id(&self) -> Self::Id;
}

pub(super) fn remap<Row, Id>(
    id: impl Fn(&Row) -> Id,
    previous: &[Row],
    next: &[Row],
    selected: &HashSet<u32>,
    anchor: Option<u32>,
) -> (HashSet<u32>, Option<u32>)
where
    Id: Clone + Eq + Hash,
{
    let named: HashSet<Id> = selected
        .iter()
        .filter_map(|&index| previous.get(index as usize).map(&id))
        .collect();
    let mut kept = HashSet::new();
    let mut new_anchor = None;
    let anchor_name = anchor.and_then(|index| previous.get(index as usize).map(&id));
    // Table row counts are far below the u32 range.
    #[allow(clippy::cast_possible_truncation)]
    for (index, row) in next.iter().enumerate() {
        let name = id(row);
        if named.contains(&name) {
            kept.insert(index as u32);
        }
        if anchor_name.as_ref() == Some(&name) {
            new_anchor = Some(index as u32);
        }
    }
    (kept, new_anchor)
}

pub(super) fn retain_in_range(
    next_len: u32,
    selected: &HashSet<u32>,
    anchor: Option<u32>,
) -> (HashSet<u32>, Option<u32>) {
    let kept = selected
        .iter()
        .copied()
        .filter(|&index| index < next_len)
        .collect();
    let anchor = anchor.filter(|&index| index < next_len);
    (kept, anchor)
}
