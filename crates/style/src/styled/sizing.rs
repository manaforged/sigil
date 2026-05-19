//! `Sizing` super-trait — width, height, min/max.

use super::StyleAccess;
use crate::{Dimension, StyleAtom};

/// Sizing methods on the fluent styling surface — width and height.
pub trait Sizing: StyleAccess {
    /// Set preferred width.
    #[must_use]
    fn w(mut self, dim: Dimension) -> Self {
        self.style().push_atom(StyleAtom::Width(dim));
        self
    }

    /// Set preferred height.
    #[must_use]
    fn h(mut self, dim: Dimension) -> Self {
        self.style().push_atom(StyleAtom::Height(dim));
        self
    }

    /// Set minimum width.
    #[must_use]
    fn min_w(mut self, dim: Dimension) -> Self {
        self.style().push_atom(StyleAtom::MinWidth(dim));
        self
    }

    /// Set maximum width.
    #[must_use]
    fn max_w(mut self, dim: Dimension) -> Self {
        self.style().push_atom(StyleAtom::MaxWidth(dim));
        self
    }
}

impl<T: StyleAccess> Sizing for T {}
