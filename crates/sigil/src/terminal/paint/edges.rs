use std::ops::Range;

pub(super) type CellEdge = [i32; 4];

/// The border owner of each cell; empty until a border is painted.
#[derive(Clone, Debug, Default)]
pub(super) struct Edges(Vec<Option<CellEdge>>);

impl Edges {
    pub(super) fn get(&self, index: usize) -> Option<CellEdge> {
        self.0.get(index).copied().flatten()
    }

    pub(super) fn set(&mut self, cells: usize, index: usize, edge: Option<CellEdge>) {
        if edge.is_some() && self.0.is_empty() {
            self.0.resize(cells, None);
        }
        if let Some(slot) = self.0.get_mut(index) {
            *slot = edge;
        }
    }

    pub(super) fn clear(&mut self) {
        self.0.clear();
    }

    pub(super) fn clear_range(&mut self, range: Range<usize>) {
        if let Some(slots) = self.0.get_mut(range) {
            slots.fill(None);
        }
    }

    pub(super) fn copy_range(&mut self, cells: usize, from: &Self, range: Range<usize>) {
        let Some(next) = from.0.get(range.clone()) else {
            self.clear_range(range);
            return;
        };
        if self.0.is_empty() {
            self.0.resize(cells, None);
        }
        if let Some(slots) = self.0.get_mut(range) {
            slots.copy_from_slice(next);
        }
    }

    fn painted(&self) -> impl Iterator<Item = (usize, CellEdge)> + '_ {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| edge.map(|edge| (index, edge)))
    }
}

impl PartialEq for Edges {
    fn eq(&self, other: &Self) -> bool {
        self.painted().eq(other.painted())
    }
}

impl Eq for Edges {}
