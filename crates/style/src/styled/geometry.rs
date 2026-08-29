use super::StyleAccess;
use crate::{Px, StyleAtom};

pub trait Geometry: StyleAccess {
    #[must_use]
    fn rounded(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::BorderRadius(Px(px)));
        self
    }

    #[must_use]
    fn border_w(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::BorderWidth(Px(px)));
        self
    }

    #[must_use]
    fn border_color(mut self, color: impl Into<super::ColorOrToken>) -> Self {
        self.style_mut()
            .push_atom(super::color::border_color_atom(color.into()));
        self
    }
}

impl<T: StyleAccess> Geometry for T {}
