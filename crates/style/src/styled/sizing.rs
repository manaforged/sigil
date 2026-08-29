use super::StyleAccess;
use crate::{Dimension, StyleAtom};

pub trait Sizing: StyleAccess {
    #[must_use]
    fn w(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::Width(dim));
        self
    }

    #[must_use]
    fn h(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::Height(dim));
        self
    }

    #[must_use]
    fn min_w(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::MinWidth(dim));
        self
    }

    #[must_use]
    fn max_w(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::MaxWidth(dim));
        self
    }

    #[must_use]
    fn min_h(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::MinHeight(dim));
        self
    }

    #[must_use]
    fn max_h(mut self, dim: Dimension) -> Self {
        self.style_mut().push_atom(StyleAtom::MaxHeight(dim));
        self
    }
}

impl<T: StyleAccess> Sizing for T {}
