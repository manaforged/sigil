use smallvec::SmallVec;

use crate::kernel::ImageSource;
use crate::style::image_paint::ImagePaints;
use crate::style::live_paint::LivePaint;
use crate::style::merge::SlotSet;
use crate::style::{Paint, StyleAtom};

/// Builds the paint for one interaction state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct StateStyleBuilder {
    atoms: SmallVec<[StyleAtom; 16]>,
    images: ImagePaints,
    live: LivePaint,
}

impl StateStyleBuilder {
    pub(crate) const fn new() -> Self {
        Self {
            atoms: SmallVec::new_const(),
            images: ImagePaints::new(),
            live: LivePaint::new(),
        }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.atoms.is_empty() && self.images.is_empty() && self.live.is_empty()
    }

    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        if !self.live.is_empty() {
            self.live.shadow(SlotSet::atom(atom));
        }
        self.atoms.push(atom);
    }

    pub(crate) fn push_live<A>(&mut self, now: Option<A>, read: impl Fn() -> A + 'static)
    where
        A: IntoIterator<Item = StyleAtom>,
    {
        let Some(atoms) = now else {
            let slots = self.live.push(read);
            self.atoms
                .retain(|atom| !SlotSet::atom(*atom).overlaps(slots));
            return;
        };
        for atom in atoms {
            self.push_atom(atom);
        }
    }

    pub(crate) fn push_image(&mut self, source: ImageSource) -> u64 {
        self.images.push(source)
    }

    pub(crate) fn take_atoms(&mut self) -> SmallVec<[StyleAtom; 16]> {
        std::mem::take(&mut self.atoms)
    }

    pub(crate) fn take_images(&mut self) -> ImagePaints {
        std::mem::take(&mut self.images)
    }

    pub(crate) fn take_live(&mut self) -> LivePaint {
        std::mem::take(&mut self.live)
    }

    #[must_use]
    pub(crate) fn slots(&self) -> SlotSet {
        SlotSet::of(&self.atoms).union(self.live.slots())
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.atoms
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (SmallVec<[StyleAtom; 16]>, ImagePaints, LivePaint) {
        (self.atoms, self.images, self.live)
    }
}

impl Paint for StateStyleBuilder {
    fn paint_mut(&mut self) -> &mut Self {
        self
    }
}

pub(crate) fn build_state_atoms<F>(configure: F) -> StateStyleBuilder
where
    F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
{
    configure(StateStyleBuilder::new())
}
