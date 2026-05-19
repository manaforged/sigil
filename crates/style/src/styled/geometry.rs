//! `Geometry` super-trait — border radius (more geometry vocabulary will
//! land here as the surface grows).

use super::StyleAccess;
use crate::{Px, StyleAtom};

/// Geometry methods on the fluent styling surface — border radius today.
pub trait Geometry: StyleAccess {
    /// Set border radius in logical pixels.
    #[must_use]
    fn rounded(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::BorderRadius(Px(px)));
        self
    }
}

impl<T: StyleAccess> Geometry for T {}
