use super::StyleAccess;
use crate::style::{AlignKind, DisplayKind, FlexDir, FlexWrapKind, JustifyKind, StyleAtom};

pub trait Flex: StyleAccess {
    #[must_use]
    fn flex(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::Flex));
        self
    }

    #[must_use]
    fn block(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::Block));
        self
    }

    #[must_use]
    fn hidden(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::None));
        self
    }

    #[must_use]
    fn flex_row(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::Row));
        self
    }

    #[must_use]
    fn flex_wrap(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexWrap(FlexWrapKind::Wrap));
        self
    }

    #[must_use]
    fn flex_nowrap(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexWrap(FlexWrapKind::NoWrap));
        self
    }

    #[must_use]
    fn flex_col(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::Column));
        self
    }

    #[must_use]
    fn flex_row_reverse(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::RowReverse));
        self
    }

    #[must_use]
    fn flex_col_reverse(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::ColumnReverse));
        self
    }

    #[must_use]
    fn items_start(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Start));
        self
    }

    #[must_use]
    fn items_center(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Center));
        self
    }

    #[must_use]
    fn items_end(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::End));
        self
    }

    #[must_use]
    fn items_stretch(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Stretch));
        self
    }

    #[must_use]
    fn justify_start(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Start));
        self
    }

    #[must_use]
    fn justify_center(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Center));
        self
    }

    #[must_use]
    fn justify_end(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::End));
        self
    }

    #[must_use]
    fn justify_between(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Between));
        self
    }

    #[must_use]
    fn justify_around(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Around));
        self
    }

    #[must_use]
    fn flex_grow(mut self, value: u16) -> Self {
        self.style_mut().push_atom(StyleAtom::FlexGrow(value));
        self
    }

    #[must_use]
    fn shrink(mut self, value: u16) -> Self {
        self.style_mut().push_atom(StyleAtom::FlexShrink(value));
        self
    }
}

impl<T: StyleAccess> Flex for T {}
