use super::StyleAccess;
use crate::{Px, StyleAtom};

pub trait Typography: StyleAccess {
    #[must_use]
    fn font_size(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::FontSize(Px(px)));
        self
    }

    #[must_use]
    fn font_weight(mut self, weight: u16) -> Self {
        self.style_mut().push_atom(StyleAtom::FontWeight(weight));
        self
    }

    #[must_use]
    fn font_italic(mut self, italic: bool) -> Self {
        self.style_mut().push_atom(StyleAtom::FontItalic(italic));
        self
    }

    #[must_use]
    fn font_family(mut self, name: &'static str) -> Self {
        self.style_mut().push_atom(StyleAtom::FontFamily(name));
        self
    }

    #[must_use]
    fn line_height(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::LineHeight(Px(px)));
        self
    }
}

impl<T: StyleAccess> Typography for T {}
