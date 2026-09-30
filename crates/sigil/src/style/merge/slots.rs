use smallvec::SmallVec;

use crate::style::StyleAtom;

use super::{ResolvedSlot, expand_atom};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct SlotSet(u64);

impl SlotSet {
    pub(crate) const EMPTY: Self = Self(0);

    pub(crate) fn of(atoms: &[StyleAtom]) -> Self {
        atoms
            .iter()
            .fold(Self::EMPTY, |set, atom| set.union(Self::atom(*atom)))
    }

    pub(crate) fn atom(atom: StyleAtom) -> Self {
        let mut expanded = SmallVec::<[(ResolvedSlot, StyleAtom); 4]>::new();
        expand_atom(atom, &mut expanded);
        expanded
            .iter()
            .fold(Self::EMPTY, |set, (slot, _)| set.with(*slot))
    }

    pub(crate) const fn with(self, slot: ResolvedSlot) -> Self {
        Self(self.0 | 1_u64 << slot as u8)
    }

    pub(crate) const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub(crate) const fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }
}
