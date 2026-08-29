use smallvec::SmallVec;

use super::StyleAccess;
use crate::{Style, StyleAtom};

pub trait StateHost {}

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

impl StyleAccess for StateStyleBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}
