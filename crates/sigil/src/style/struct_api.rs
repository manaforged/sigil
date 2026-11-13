use smallvec::SmallVec;

use crate::style::image_paint::ImagePaints;
use crate::style::merge::{SlotSet, merge_classes};
use crate::style::state::PerState;
use crate::style::{Paint, StateSlot, StateStyleBuilder, StyleAtom, StyleOverlay};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Style {
    pub(crate) paint: StateStyleBuilder,
    states: PerState<SmallVec<[StyleAtom; 4]>>,
}

impl Style {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            paint: StateStyleBuilder::new(),
            states: PerState::new(
                SmallVec::new_const(),
                SmallVec::new_const(),
                SmallVec::new_const(),
                SmallVec::new_const(),
                SmallVec::new_const(),
            ),
        }
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.paint.into_atoms()
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.paint.is_empty() && !self.has_state_overlays()
    }

    #[must_use]
    pub(crate) fn has_state_overlays(&self) -> bool {
        self.states.each().iter().any(|atoms| !atoms.is_empty())
    }

    pub(crate) fn set_state(&mut self, slot: StateSlot, atoms: SmallVec<[StyleAtom; 16]>) {
        *self.states.get_mut(slot) = atoms.into_iter().collect();
    }

    #[must_use]
    pub(crate) fn take_state_overlay(&mut self) -> StyleOverlay {
        StyleOverlay::from_states(std::mem::take(&mut self.states))
    }

    pub(crate) fn take_atoms(&mut self) -> SmallVec<[StyleAtom; 16]> {
        self.paint.take_atoms()
    }

    pub(crate) fn take_images(&mut self) -> ImagePaints {
        self.paint.take_images()
    }

    pub(crate) fn merge_under(&mut self, base: &Self) {
        let caller_slots = SlotSet::of(self.paint.atoms());
        for (caller_slot, base_slot) in self.states.each_mut().into_iter().zip(base.states.each()) {
            let kept_base: SmallVec<[StyleAtom; 4]> = base_slot
                .iter()
                .copied()
                .filter(|atom| {
                    caller_slots.is_empty() || !SlotSet::atom(*atom).overlaps(caller_slots)
                })
                .collect();
            *caller_slot = merge_classes(&kept_base, caller_slot, 0)
                .into_iter()
                .collect();
        }
    }

    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        self.paint.push_atom(atom);
    }

    #[must_use]
    pub(crate) fn radius_px(mut self, px: i16) -> Self {
        self.push_atom(StyleAtom::BorderRadius(px));
        self
    }
}

impl Paint for Style {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        &mut self.paint
    }
}
