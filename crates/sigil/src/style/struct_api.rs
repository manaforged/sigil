use smallvec::SmallVec;

use crate::style::image_paint::ImagePaints;
use crate::style::live_paint::{LivePaint, NO_STATE_PAINT};
use crate::style::merge::SlotSet;
use crate::style::overlay::merge_state;
use crate::style::state::PerState;
use crate::style::{Paint, StateSlot, StateStyleBuilder, StyleAtom, StyleOverlay};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Style {
    pub(crate) paint: StateStyleBuilder,
    states: PerState<SmallVec<[StyleAtom; 4]>>,
    live_states: PerState<LivePaint>,
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
            live_states: NO_STATE_PAINT,
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
            || self.live_states.each().iter().any(|live| !live.is_empty())
    }

    pub(crate) fn set_state(&mut self, slot: StateSlot, paint: StateStyleBuilder) {
        let (atoms, _images, live) = paint.into_parts();
        *self.states.get_mut(slot) = atoms.into_iter().collect();
        *self.live_states.get_mut(slot) = live;
    }

    #[must_use]
    pub(crate) fn take_state_overlay(&mut self) -> StyleOverlay {
        StyleOverlay::from_states(std::mem::take(&mut self.states))
            .live(std::mem::take(&mut self.live_states))
    }

    pub(crate) fn take_atoms(&mut self) -> SmallVec<[StyleAtom; 16]> {
        self.paint.take_atoms()
    }

    pub(crate) fn take_images(&mut self) -> ImagePaints {
        self.paint.take_images()
    }

    pub(crate) fn take_shaders(&mut self) -> crate::style::shader_paint::ShaderPaints {
        self.paint.take_shaders()
    }

    pub(crate) fn take_live(&mut self) -> LivePaint {
        self.paint.take_live()
    }

    pub(crate) fn merge_under(&mut self, base: &Self) {
        let caller_slots = self.paint.slots();
        let callers = self
            .states
            .each_mut()
            .into_iter()
            .zip(self.live_states.each_mut());
        let bases = base.states.each().into_iter().zip(base.live_states.each());
        for ((caller_atoms, caller_live), (base_atoms, base_live)) in callers.zip(bases) {
            let mut atoms: SmallVec<[StyleAtom; 4]> = base_atoms
                .iter()
                .copied()
                .filter(|atom| !SlotSet::atom(*atom).overlaps(caller_slots))
                .collect();
            let mut live = base_live.clone();
            live.shadow(caller_slots);
            merge_state(&mut atoms, &mut live, caller_atoms, caller_live);
            *caller_atoms = atoms;
            *caller_live = live;
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
