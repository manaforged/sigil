use super::StyleAccess;
use crate::{Px, StyleAtom};

pub trait Spacing: StyleAccess {
    #[must_use]
    fn padding(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingAll(Px(px)));
        self
    }

    #[must_use]
    fn px(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingX(Px(px)));
        self
    }

    #[must_use]
    fn py(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingY(Px(px)));
        self
    }

    #[must_use]
    fn pt(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingTop(Px(px)));
        self
    }

    #[must_use]
    fn pr(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingRight(Px(px)));
        self
    }

    #[must_use]
    fn pb(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingBottom(Px(px)));
        self
    }

    #[must_use]
    fn pl(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::PaddingLeft(Px(px)));
        self
    }

    #[must_use]
    fn margin(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginAll(Px(px)));
        self
    }

    #[must_use]
    fn mx(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginX(Px(px)));
        self
    }

    #[must_use]
    fn my(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginY(Px(px)));
        self
    }

    #[must_use]
    fn mt(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginTop(Px(px)));
        self
    }

    #[must_use]
    fn mr(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginRight(Px(px)));
        self
    }

    #[must_use]
    fn mb(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginBottom(Px(px)));
        self
    }

    #[must_use]
    fn ml(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::MarginLeft(Px(px)));
        self
    }

    #[must_use]
    fn gap(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::Gap(Px(px)));
        self
    }

    #[must_use]
    fn gap_x(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::GapX(Px(px)));
        self
    }

    #[must_use]
    fn gap_y(mut self, px: i16) -> Self {
        self.style_mut().push_atom(StyleAtom::GapY(Px(px)));
        self
    }
}

impl<T: StyleAccess> Spacing for T {}
