use crate::layout::Space;
use crate::layout::style_data::{Axes, Edges, Length, Overflow};

use super::{LayoutFields, LayoutStyle};

const PADDING: LayoutFields = LayoutFields::PADDING_TOP
    .with(LayoutFields::PADDING_RIGHT)
    .with(LayoutFields::PADDING_BOTTOM)
    .with(LayoutFields::PADDING_LEFT);
const INSET: LayoutFields = LayoutFields::INSET_TOP
    .with(LayoutFields::INSET_RIGHT)
    .with(LayoutFields::INSET_BOTTOM)
    .with(LayoutFields::INSET_LEFT);
const MARGIN: LayoutFields = LayoutFields::MARGIN_TOP
    .with(LayoutFields::MARGIN_RIGHT)
    .with(LayoutFields::MARGIN_BOTTOM)
    .with(LayoutFields::MARGIN_LEFT);

impl LayoutStyle {
    /// Sets the padding on all four sides to a step of the spacing scale.
    #[must_use]
    pub fn padding(mut self, padding: impl Into<Space>) -> Self {
        self.0.padding = Edges::splat(Length::from(padding.into()));
        self.mark(PADDING)
    }

    /// Sets the left and right padding.
    #[must_use]
    pub fn padding_x(mut self, padding: impl Into<Space>) -> Self {
        let padding = Length::from(padding.into());
        (self.0.padding.left, self.0.padding.right) = (padding, padding);
        self.mark(LayoutFields::PADDING_LEFT.with(LayoutFields::PADDING_RIGHT))
    }

    /// Sets the top and bottom padding.
    #[must_use]
    pub fn padding_y(mut self, padding: impl Into<Space>) -> Self {
        let padding = Length::from(padding.into());
        (self.0.padding.top, self.0.padding.bottom) = (padding, padding);
        self.mark(LayoutFields::PADDING_TOP.with(LayoutFields::PADDING_BOTTOM))
    }

    /// Sets the padding on the top side.
    #[must_use]
    pub fn padding_top(mut self, padding: impl Into<Space>) -> Self {
        self.0.padding.top = Length::from(padding.into());
        self.mark(LayoutFields::PADDING_TOP)
    }

    /// Sets the padding on the right side.
    #[must_use]
    pub fn padding_right(mut self, padding: impl Into<Space>) -> Self {
        self.0.padding.right = Length::from(padding.into());
        self.mark(LayoutFields::PADDING_RIGHT)
    }

    /// Sets the padding on the bottom side.
    #[must_use]
    pub fn padding_bottom(mut self, padding: impl Into<Space>) -> Self {
        self.0.padding.bottom = Length::from(padding.into());
        self.mark(LayoutFields::PADDING_BOTTOM)
    }

    /// Sets the padding on the left side.
    #[must_use]
    pub fn padding_left(mut self, padding: impl Into<Space>) -> Self {
        self.0.padding.left = Length::from(padding.into());
        self.mark(LayoutFields::PADDING_LEFT)
    }

    /// Sets the overflow mode on the horizontal axis.
    #[must_use]
    pub const fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.0.overflow.width = overflow;
        self.mark(LayoutFields::OVERFLOW_X)
    }

    /// Sets the overflow mode on the vertical axis.
    #[must_use]
    pub const fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.0.overflow.height = overflow;
        self.mark(LayoutFields::OVERFLOW_Y)
    }

    /// Sets the overflow mode on both axes.
    #[must_use]
    pub const fn overflow(self, overflow: Overflow) -> Self {
        self.overflow_x(overflow).overflow_y(overflow)
    }

    /// Sets the inset from the top edge.
    #[must_use]
    pub fn inset_top(mut self, top: impl Into<Length>) -> Self {
        self.0.inset.top = top.into();
        self.mark(LayoutFields::INSET_TOP)
    }

    /// Sets the inset from the left edge.
    #[must_use]
    pub fn inset_left(mut self, left: impl Into<Length>) -> Self {
        self.0.inset.left = left.into();
        self.mark(LayoutFields::INSET_LEFT)
    }

    /// Sets the inset from the right edge.
    #[must_use]
    pub fn inset_right(mut self, right: impl Into<Length>) -> Self {
        self.0.inset.right = right.into();
        self.mark(LayoutFields::INSET_RIGHT)
    }

    /// Sets the inset from the bottom edge.
    #[must_use]
    pub fn inset_bottom(mut self, bottom: impl Into<Length>) -> Self {
        self.0.inset.bottom = bottom.into();
        self.mark(LayoutFields::INSET_BOTTOM)
    }

    /// Sets the inset from all four edges.
    #[must_use]
    pub fn inset(mut self, inset: impl Into<Length>) -> Self {
        self.0.inset = Edges::splat(inset.into());
        self.mark(INSET)
    }

    /// Sets the gap between children on both axes to a step of the spacing scale.
    #[must_use]
    pub fn gap(mut self, gap: impl Into<Space>) -> Self {
        self.0.gap = Axes::splat(Length::from(gap.into()));
        self.mark(LayoutFields::GAP_X.with(LayoutFields::GAP_Y))
    }

    /// Sets the margin on all four sides to a step of the spacing scale.
    #[must_use]
    pub fn margin(mut self, margin: impl Into<Space>) -> Self {
        self.0.margin = Edges::splat(Length::from(margin.into()));
        self.mark(MARGIN)
    }

    /// Sets the left and right margin.
    #[must_use]
    pub fn margin_x(mut self, margin: impl Into<Space>) -> Self {
        let margin = Length::from(margin.into());
        (self.0.margin.left, self.0.margin.right) = (margin, margin);
        self.mark(LayoutFields::MARGIN_LEFT.with(LayoutFields::MARGIN_RIGHT))
    }

    /// Sets the top and bottom margin.
    #[must_use]
    pub fn margin_y(mut self, margin: impl Into<Space>) -> Self {
        let margin = Length::from(margin.into());
        (self.0.margin.top, self.0.margin.bottom) = (margin, margin);
        self.mark(LayoutFields::MARGIN_TOP.with(LayoutFields::MARGIN_BOTTOM))
    }

    /// Sets the margin on the top side.
    #[must_use]
    pub fn margin_top(mut self, margin: impl Into<Space>) -> Self {
        self.0.margin.top = Length::from(margin.into());
        self.mark(LayoutFields::MARGIN_TOP)
    }

    /// Sets the margin on the right side.
    #[must_use]
    pub fn margin_right(mut self, margin: impl Into<Space>) -> Self {
        self.0.margin.right = Length::from(margin.into());
        self.mark(LayoutFields::MARGIN_RIGHT)
    }

    /// Sets the margin on the bottom side.
    #[must_use]
    pub fn margin_bottom(mut self, margin: impl Into<Space>) -> Self {
        self.0.margin.bottom = Length::from(margin.into());
        self.mark(LayoutFields::MARGIN_BOTTOM)
    }

    /// Sets the margin on the left side.
    #[must_use]
    pub fn margin_left(mut self, margin: impl Into<Space>) -> Self {
        self.0.margin.left = Length::from(margin.into());
        self.mark(LayoutFields::MARGIN_LEFT)
    }

    /// Sets the horizontal gap between children.
    #[must_use]
    pub fn gap_x(mut self, gap: impl Into<Space>) -> Self {
        self.0.gap.width = Length::from(gap.into());
        self.mark(LayoutFields::GAP_X)
    }

    /// Sets the vertical gap between children.
    #[must_use]
    pub fn gap_y(mut self, gap: impl Into<Space>) -> Self {
        self.0.gap.height = Length::from(gap.into());
        self.mark(LayoutFields::GAP_Y)
    }
}
