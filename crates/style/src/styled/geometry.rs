//! `Geometry` super-trait — border radius (more geometry vocabulary will
//! land here as the surface grows).

use super::StyleAccess;
use crate::{Px, StyleAtom};

/// Geometry methods on the fluent styling surface — border radius today.
pub trait Geometry: StyleAccess {
    /// Set border radius in logical pixels.
    #[must_use]
    fn rounded(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::BorderRadius(Px(px)));
        self
    }

    /// Set border stroke width in logical pixels (all four sides).
    #[must_use]
    fn border_w(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::BorderWidth(Px(px)));
        self
    }

    /// Set border stroke colour.
    ///
    /// Accepts [`crate::Rgba8`], `&'static str` (theme token), or
    /// [`crate::ColorAtom`].
    #[must_use]
    fn border_color(mut self, color: impl Into<super::ColorOrToken>) -> Self {
        self.style_mut()
            .push_atom(super::color::border_color_atom(color.into()));
        self
    }
}

impl<T: StyleAccess> Geometry for T {}
