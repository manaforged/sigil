//! `Typography` super-trait — font size, weight, family, line height.

use super::StyleAccess;
use crate::{Px, StyleAtom};

/// Typography methods on the fluent styling surface.
pub trait Typography: StyleAccess {
    /// Set font size in logical pixels.
    #[must_use]
    fn font_size(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::FontSize(Px(px)));
        self
    }

    /// Set font weight (CSS scale: 100..=900).
    #[must_use]
    fn font_weight(mut self, weight: u16) -> Self {
        self.style().push_atom(StyleAtom::FontWeight(weight));
        self
    }

    /// Set font family name.
    #[must_use]
    fn font_family(mut self, name: &'static str) -> Self {
        self.style().push_atom(StyleAtom::FontFamily(name));
        self
    }

    /// Set line height in logical pixels.
    #[must_use]
    fn line_height(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::LineHeight(Px(px)));
        self
    }
}

impl<T: StyleAccess> Typography for T {}
