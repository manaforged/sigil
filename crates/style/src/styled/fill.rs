use super::ColorOrToken;
use super::StyleAccess;
use super::color::{background_atom, text_atom};

pub trait Fill: StyleAccess {
    #[must_use]
    fn bg(mut self, color: impl Into<ColorOrToken>) -> Self {
        self.style_mut().push_atom(background_atom(color.into()));
        self
    }

    #[must_use]
    fn text_color(mut self, color: impl Into<ColorOrToken>) -> Self {
        self.style_mut().push_atom(text_atom(color.into()));
        self
    }
}

impl<T: StyleAccess> Fill for T {}
