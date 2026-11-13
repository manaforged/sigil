use smallvec::SmallVec;

use crate::kernel::ImageSource;
use crate::style::image_paint::ImagePaints;
use crate::style::{Paint, StyleAtom};

/// Builds the paint for one interaction state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct StateStyleBuilder {
    atoms: SmallVec<[StyleAtom; 16]>,
    images: ImagePaints,
}

impl StateStyleBuilder {
    pub(crate) const fn new() -> Self {
        Self {
            atoms: SmallVec::new_const(),
            images: ImagePaints::new(),
        }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.atoms.is_empty() && self.images.is_empty()
    }

    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        self.atoms.push(atom);
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

    #[must_use]
    pub(crate) fn atoms(&self) -> &[StyleAtom] {
        &self.atoms
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.atoms
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (SmallVec<[StyleAtom; 16]>, ImagePaints) {
        (self.atoms, self.images)
    }
}

impl Paint for StateStyleBuilder {
    fn paint_mut(&mut self) -> &mut Self {
        self
    }
}

pub(crate) fn build_state_atoms<F>(configure: F) -> SmallVec<[StyleAtom; 16]>
where
    F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
{
    configure(StateStyleBuilder::new()).into_atoms()
}
