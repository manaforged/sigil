use smallvec::SmallVec;

use crate::style::image_paint::ImagePaints;
use crate::style::{PaintTarget, Style, StyleAtom, StyleSlot};

/// Builds the paint for one interaction state.
#[derive(Debug)]
#[must_use]
pub struct StateStyleBuilder {
    style: Style,
}

impl StateStyleBuilder {
    pub(crate) fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.style.into_atoms()
    }

    pub(crate) fn into_parts(mut self) -> (SmallVec<[StyleAtom; 16]>, ImagePaints) {
        let images = self.style.take_images();
        (self.style.into_atoms(), images)
    }
}

impl PaintTarget for StateStyleBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style)
    }
}
