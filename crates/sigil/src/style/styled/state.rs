use smallvec::SmallVec;

use crate::style::{PaintTarget, Style, StyleAtom, StyleSlot};

pub struct StateStyleBuilder {
    style: Style,
}

impl StateStyleBuilder {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.style.into_atoms()
    }
}

impl PaintTarget for StateStyleBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style)
    }
}
