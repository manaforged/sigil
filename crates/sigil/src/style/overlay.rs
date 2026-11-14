use smallvec::SmallVec;

use crate::style::live_paint::{LivePaint, NO_STATE_PAINT};
use crate::style::merge::SlotSet;
use crate::style::state::PerState;
use crate::style::{StateSlot, StyleAtom};

/// Holds extra style atoms for each interaction state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StyleOverlay {
    slots: PerState<SmallVec<[StyleAtom; 4]>>,
    live: PerState<LivePaint>,
}

impl StyleOverlay {
    #[cfg(test)]
    /// Creates a style overlay.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    /// Creates a style overlay from slots.
    #[must_use]
    pub(crate) const fn from_slots(
        rest: SmallVec<[StyleAtom; 4]>,
        hover: SmallVec<[StyleAtom; 4]>,
        focus: SmallVec<[StyleAtom; 4]>,
        pressed: SmallVec<[StyleAtom; 4]>,
        disabled: SmallVec<[StyleAtom; 4]>,
    ) -> Self {
        Self::from_states(PerState::new(rest, focus, hover, pressed, disabled))
    }

    #[must_use]
    pub(crate) const fn from_states(slots: PerState<SmallVec<[StyleAtom; 4]>>) -> Self {
        Self {
            slots,
            live: NO_STATE_PAINT,
        }
    }

    #[must_use]
    pub(crate) fn live(mut self, live: PerState<LivePaint>) -> Self {
        self.live = live;
        self
    }

    pub(crate) fn take_live(&mut self) -> PerState<LivePaint> {
        std::mem::take(&mut self.live)
    }

    /// Returns whether this style overlay is empty.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.each().iter().all(|atoms| atoms.is_empty())
            && self.live.each().iter().all(|live| live.is_empty())
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        let into = self.slots.each_mut().into_iter().zip(self.live.each_mut());
        let from = other.slots.each().into_iter().zip(other.live.each());
        for ((into_atoms, into_live), (from_atoms, from_live)) in into.zip(from) {
            merge_state(into_atoms, into_live, from_atoms, from_live);
        }
    }

    /// Returns the atoms of one state slot.
    #[must_use]
    pub(crate) fn atoms_for(&self, slot: StateSlot) -> &[StyleAtom] {
        self.slots.get(slot)
    }

    #[must_use]
    pub(crate) fn any(&self, pred: fn(StyleAtom) -> bool) -> bool {
        self.slots.each().into_iter().flatten().copied().any(pred)
    }

    #[must_use]
    pub(crate) fn may_transform(&self) -> bool {
        self.any(StyleAtom::may_transform)
    }
}

pub(crate) fn merge_state(
    atoms: &mut SmallVec<[StyleAtom; 4]>,
    live: &mut LivePaint,
    over: &[StyleAtom],
    over_live: &LivePaint,
) {
    live.shadow(SlotSet::of(over));
    let shadowed = over_live.slots();
    *atoms = crate::style::merge_classes(atoms, over, 0)
        .into_iter()
        .filter(|atom| !SlotSet::atom(*atom).overlaps(shadowed))
        .collect();
    live.append(over_live);
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
