//! `Flex` super-trait — display mode, flex direction, alignment,
//! justification, grow/shrink.

use super::StyleAccess;
use crate::{AlignKind, DisplayKind, FlexDir, JustifyKind, StyleAtom};

/// Display + flex methods on the fluent styling surface.
pub trait Flex: StyleAccess {
    /// Use flex layout.
    #[must_use]
    fn flex(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::Flex));
        self
    }

    /// Use block layout.
    #[must_use]
    fn block(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::Block));
        self
    }

    /// Hide the node from layout and paint.
    #[must_use]
    fn hidden(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::Display(DisplayKind::None));
        self
    }

    /// Set flex direction to row.
    #[must_use]
    fn flex_row(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::Row));
        self
    }

    /// Set flex direction to column.
    #[must_use]
    fn flex_col(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::Column));
        self
    }

    /// Set flex direction to reverse row.
    #[must_use]
    fn flex_row_reverse(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::RowReverse));
        self
    }

    /// Set flex direction to reverse column.
    #[must_use]
    fn flex_col_reverse(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::FlexDirection(FlexDir::ColumnReverse));
        self
    }

    /// Align items to the cross-axis start.
    #[must_use]
    fn items_start(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Start));
        self
    }

    /// Align items to the cross-axis centre.
    #[must_use]
    fn items_center(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Center));
        self
    }

    /// Align items to the cross-axis end.
    #[must_use]
    fn items_end(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::End));
        self
    }

    /// Stretch items across the cross axis.
    #[must_use]
    fn items_stretch(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::AlignItems(AlignKind::Stretch));
        self
    }

    /// Justify content at the main-axis start.
    #[must_use]
    fn justify_start(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Start));
        self
    }

    /// Justify content at the main-axis centre.
    #[must_use]
    fn justify_center(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Center));
        self
    }

    /// Justify content at the main-axis end.
    #[must_use]
    fn justify_end(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::End));
        self
    }

    /// Distribute content with space between items.
    #[must_use]
    fn justify_between(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Between));
        self
    }

    /// Distribute content with space around items.
    #[must_use]
    fn justify_around(mut self) -> Self {
        self.style_mut()
            .push_atom(StyleAtom::JustifyContent(JustifyKind::Around));
        self
    }

    /// Set flex grow factor (encoded as `value * 1000`).
    ///
    /// Named `flex_grow` (CSS canonical) rather than `grow` to avoid the
    /// silent type-confusion footgun with [`crate::Style::flex_grow`] on
    /// the lower-level layout primitive — both used to be named `grow`
    /// but one takes `u16` (this one) and the other takes `f32`.
    #[must_use]
    fn flex_grow(mut self, value: u16) -> Self {
        self.style_mut().push_atom(StyleAtom::FlexGrow(value));
        self
    }

    /// Set flex shrink factor (encoded as `value * 1000`).
    #[must_use]
    fn shrink(mut self, value: u16) -> Self {
        self.style_mut().push_atom(StyleAtom::FlexShrink(value));
        self
    }
}

impl<T: StyleAccess> Flex for T {}
