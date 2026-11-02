use crate::layout::style_data::{Axes, Dimension, Edges, Overflow};

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
    /// Sets the padding.
    #[must_use]
    pub fn padding(mut self, padding: impl Into<Dimension>) -> Self {
        self.0.padding = Edges::splat(padding.into());
        self.mark(PADDING)
    }

    /// Sets the padding x.
    #[must_use]
    pub fn padding_x(mut self, padding: impl Into<Dimension>) -> Self {
        let padding = padding.into();
        (self.0.padding.left, self.0.padding.right) = (padding, padding);
        self.mark(LayoutFields::PADDING_LEFT.with(LayoutFields::PADDING_RIGHT))
    }

    /// Sets the padding y.
    #[must_use]
    pub fn padding_y(mut self, padding: impl Into<Dimension>) -> Self {
        let padding = padding.into();
        (self.0.padding.top, self.0.padding.bottom) = (padding, padding);
        self.mark(LayoutFields::PADDING_TOP.with(LayoutFields::PADDING_BOTTOM))
    }

    /// Sets the padding top.
    #[must_use]
    pub fn padding_top(mut self, padding: impl Into<Dimension>) -> Self {
        self.0.padding.top = padding.into();
        self.mark(LayoutFields::PADDING_TOP)
    }

    /// Sets the padding right.
    #[must_use]
    pub fn padding_right(mut self, padding: impl Into<Dimension>) -> Self {
        self.0.padding.right = padding.into();
        self.mark(LayoutFields::PADDING_RIGHT)
    }

    /// Sets the padding bottom.
    #[must_use]
    pub fn padding_bottom(mut self, padding: impl Into<Dimension>) -> Self {
        self.0.padding.bottom = padding.into();
        self.mark(LayoutFields::PADDING_BOTTOM)
    }

    /// Sets the padding left.
    #[must_use]
    pub fn padding_left(mut self, padding: impl Into<Dimension>) -> Self {
        self.0.padding.left = padding.into();
        self.mark(LayoutFields::PADDING_LEFT)
    }

    /// Sets the overflow x.
    #[must_use]
    pub const fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.0.overflow.width = overflow;
        self.mark(LayoutFields::OVERFLOW_X)
    }

    /// Sets the overflow y.
    #[must_use]
    pub const fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.0.overflow.height = overflow;
        self.mark(LayoutFields::OVERFLOW_Y)
    }

    /// Sets the overflow.
    #[must_use]
    pub const fn overflow(self, overflow: Overflow) -> Self {
        self.overflow_x(overflow).overflow_y(overflow)
    }

    /// Sets the inset top.
    #[must_use]
    pub fn inset_top(mut self, top: impl Into<Dimension>) -> Self {
        self.0.inset.top = top.into();
        self.mark(LayoutFields::INSET_TOP)
    }

    /// Sets the inset left.
    #[must_use]
    pub fn inset_left(mut self, left: impl Into<Dimension>) -> Self {
        self.0.inset.left = left.into();
        self.mark(LayoutFields::INSET_LEFT)
    }

    /// Sets the inset right.
    #[must_use]
    pub fn inset_right(mut self, right: impl Into<Dimension>) -> Self {
        self.0.inset.right = right.into();
        self.mark(LayoutFields::INSET_RIGHT)
    }

    /// Sets the inset bottom.
    #[must_use]
    pub fn inset_bottom(mut self, bottom: impl Into<Dimension>) -> Self {
        self.0.inset.bottom = bottom.into();
        self.mark(LayoutFields::INSET_BOTTOM)
    }

    /// Sets the inset.
    #[must_use]
    pub fn inset(mut self, inset: impl Into<Dimension>) -> Self {
        self.0.inset = Edges::splat(inset.into());
        self.mark(INSET)
    }

    /// Sets the gap.
    #[must_use]
    pub fn gap(mut self, gap: impl Into<Dimension>) -> Self {
        self.0.gap = Axes::splat(gap.into());
        self.mark(LayoutFields::GAP_X.with(LayoutFields::GAP_Y))
    }

    /// Sets the margin.
    #[must_use]
    pub fn margin(mut self, margin: impl Into<Dimension>) -> Self {
        self.0.margin = Edges::splat(margin.into());
        self.mark(MARGIN)
    }

    /// Sets the margin x.
    #[must_use]
    pub fn margin_x(mut self, margin: impl Into<Dimension>) -> Self {
        let margin = margin.into();
        (self.0.margin.left, self.0.margin.right) = (margin, margin);
        self.mark(LayoutFields::MARGIN_LEFT.with(LayoutFields::MARGIN_RIGHT))
    }

    /// Sets the margin y.
    #[must_use]
    pub fn margin_y(mut self, margin: impl Into<Dimension>) -> Self {
        let margin = margin.into();
        (self.0.margin.top, self.0.margin.bottom) = (margin, margin);
        self.mark(LayoutFields::MARGIN_TOP.with(LayoutFields::MARGIN_BOTTOM))
    }

    /// Sets the margin top.
    #[must_use]
    pub fn margin_top(mut self, margin: impl Into<Dimension>) -> Self {
        self.0.margin.top = margin.into();
        self.mark(LayoutFields::MARGIN_TOP)
    }

    /// Sets the margin right.
    #[must_use]
    pub fn margin_right(mut self, margin: impl Into<Dimension>) -> Self {
        self.0.margin.right = margin.into();
        self.mark(LayoutFields::MARGIN_RIGHT)
    }

    /// Sets the margin bottom.
    #[must_use]
    pub fn margin_bottom(mut self, margin: impl Into<Dimension>) -> Self {
        self.0.margin.bottom = margin.into();
        self.mark(LayoutFields::MARGIN_BOTTOM)
    }

    /// Sets the margin left.
    #[must_use]
    pub fn margin_left(mut self, margin: impl Into<Dimension>) -> Self {
        self.0.margin.left = margin.into();
        self.mark(LayoutFields::MARGIN_LEFT)
    }

    /// Sets the gap x.
    #[must_use]
    pub fn gap_x(mut self, gap: impl Into<Dimension>) -> Self {
        self.0.gap.width = gap.into();
        self.mark(LayoutFields::GAP_X)
    }

    /// Sets the gap y.
    #[must_use]
    pub fn gap_y(mut self, gap: impl Into<Dimension>) -> Self {
        self.0.gap.height = gap.into();
        self.mark(LayoutFields::GAP_Y)
    }
}
