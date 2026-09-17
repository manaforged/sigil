use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

pub trait Identifiable {
    type Id: Clone + Eq + Hash;

    fn id(&self) -> Self::Id;
}

impl<Row: Identifiable> Identifiable for Arc<Row> {
    type Id = Row::Id;

    fn id(&self) -> Self::Id {
        self.as_ref().id()
    }
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
    let anchor_name = anchor.and_then(|index| previous.get(index as usize).map(&id));
    if named.is_empty() && anchor_name.is_none() {
        return (HashSet::new(), None);
    }
    let single = named.iter().next().filter(|_| named.len() == 1);
    let mut kept = HashSet::new();
    let mut new_anchor = None;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    for (index, row) in next.iter().enumerate() {
        let name = id(row);
        if single.map_or_else(|| named.contains(&name), |only| only == &name) {
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
