//! `Spacing` super-trait — padding, margin, gap.
//!
//! Blanket impl over [`StyleAccess`] gives every styling implementor the
//! full spacing vocabulary for free.

use super::StyleAccess;
use crate::{Px, StyleAtom};

/// Spacing methods on the fluent styling surface — padding, margin, gap.
pub trait Spacing: StyleAccess {
    /// Set uniform padding in logical pixels.
    #[must_use]
    fn padding(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingAll(Px(px)));
        self
    }

    /// Set horizontal padding in logical pixels.
    #[must_use]
    fn px(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingX(Px(px)));
        self
    }

    /// Set vertical padding in logical pixels.
    #[must_use]
    fn py(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingY(Px(px)));
        self
    }

    /// Set top padding in logical pixels.
    #[must_use]
    fn pt(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingTop(Px(px)));
        self
    }

    /// Set right padding in logical pixels.
    #[must_use]
    fn pr(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingRight(Px(px)));
        self
    }

    /// Set bottom padding in logical pixels.
    #[must_use]
    fn pb(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingBottom(Px(px)));
        self
    }

    /// Set left padding in logical pixels.
    #[must_use]
    fn pl(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::PaddingLeft(Px(px)));
        self
    }

    /// Set uniform margin in logical pixels.
    #[must_use]
    fn margin(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginAll(Px(px)));
        self
    }

    /// Set horizontal margin in logical pixels.
    #[must_use]
    fn mx(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginX(Px(px)));
        self
    }

    /// Set vertical margin in logical pixels.
    #[must_use]
    fn my(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginY(Px(px)));
        self
    }

    /// Set top margin in logical pixels.
    #[must_use]
    fn mt(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginTop(Px(px)));
        self
    }

    /// Set right margin in logical pixels.
    #[must_use]
    fn mr(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginRight(Px(px)));
        self
    }

    /// Set bottom margin in logical pixels.
    #[must_use]
    fn mb(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginBottom(Px(px)));
        self
    }

    /// Set left margin in logical pixels.
    #[must_use]
    fn ml(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::MarginLeft(Px(px)));
        self
    }

    /// Set flex/grid gap in logical pixels.
    #[must_use]
    fn gap(mut self, px: i16) -> Self {
        self.style().push_atom(StyleAtom::Gap(Px(px)));
        self
    }
}

impl<T: StyleAccess> Spacing for T {}
