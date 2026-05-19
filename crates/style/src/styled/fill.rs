//! `Fill` super-trait — background and text colour (polymorphic over
//! [`ColorOrToken`]).

use super::ColorOrToken;
use super::StyleAccess;
use super::color::{background_atom, text_atom};

/// Colour-fill methods on the fluent styling surface — background and
/// text colour. Accepts [`crate::Rgba8`], `&'static str` (theme token),
/// or [`crate::ColorAtom`].
pub trait Fill: StyleAccess {
    /// Set background fill.
    ///
    /// Accepts [`crate::Rgba8`], `&'static str` (theme token), or
    /// [`crate::ColorAtom`].
    #[must_use]
    fn bg(mut self, color: impl Into<ColorOrToken>) -> Self {
        self.style().push_atom(background_atom(color.into()));
        self
    }

    /// Set text fill.
    ///
    /// Accepts [`crate::Rgba8`], `&'static str` (theme token), or
    /// [`crate::ColorAtom`].
    #[must_use]
    fn text_color(mut self, color: impl Into<ColorOrToken>) -> Self {
        self.style().push_atom(text_atom(color.into()));
        self
    }
}

impl<T: StyleAccess> Fill for T {}
