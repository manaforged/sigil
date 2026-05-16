//! Layout intent attached to `sigil-kernel` nodes.

use taffy::geometry::{Rect as TaffyRect, Size as TaffySize};
use taffy::prelude::{
    AlignItems, AlignSelf, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Position, Style,
};

/// Layout intent attached to a node through `NodeExtensions`.
///
/// This is a thin newtype over `taffy::Style`; keeping the wrapper at
/// the public boundary preserves the adapter as the swap point if
/// Taffy churns or a measured fast path appears later.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutStyle(pub(crate) Style);

impl LayoutStyle {
    /// Build a block layout intent.
    #[must_use]
    pub fn block() -> Self {
        let mut style = Style {
            display: Display::Block,
            ..Style::default()
        };
        style.flex_direction = FlexDirection::Column;
        Self(style)
    }

    /// Build a row-oriented flex layout intent.
    #[must_use]
    pub fn flex_row() -> Self {
        Self(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..Style::default()
        })
    }

    /// Build a column-oriented flex layout intent.
    #[must_use]
    pub fn flex_column() -> Self {
        Self(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Style::default()
        })
    }

    /// Build a grid layout intent.
    #[must_use]
    pub fn grid() -> Self {
        Self(Style {
            display: Display::Grid,
            ..Style::default()
        })
    }

    /// Set the preferred width.
    #[must_use]
    pub const fn width(mut self, width: Dimension) -> Self {
        self.0.size.width = width;
        self
    }

    /// Set a fixed logical-pixel width.
    #[must_use]
    pub const fn width_px(self, width: f32) -> Self {
        self.width(Dimension::length(width))
    }

    /// Set the preferred height.
    #[must_use]
    pub const fn height(mut self, height: Dimension) -> Self {
        self.0.size.height = height;
        self
    }

    /// Set a fixed logical-pixel height.
    #[must_use]
    pub const fn height_px(self, height: f32) -> Self {
        self.height(Dimension::length(height))
    }

    /// Set a fixed logical-pixel width and height.
    #[must_use]
    pub const fn fixed_size(self, width: f32, height: f32) -> Self {
        self.width_px(width).height_px(height)
    }

    /// Set uniform padding.
    #[must_use]
    pub const fn padding(mut self, padding: LengthPercentage) -> Self {
        self.0.padding = all_sides(padding);
        self
    }

    /// Set fixed logical-pixel uniform padding.
    #[must_use]
    pub const fn padding_px(self, padding: f32) -> Self {
        self.padding(LengthPercentage::length(padding))
    }

    /// Set uniform flex/grid gap.
    #[must_use]
    pub const fn gap(mut self, gap: LengthPercentage) -> Self {
        self.0.gap = TaffySize {
            width: gap,
            height: gap,
        };
        self
    }

    /// Set a fixed logical-pixel flex/grid gap.
    #[must_use]
    pub const fn gap_px(self, gap: f32) -> Self {
        self.gap(LengthPercentage::length(gap))
    }

    /// Set flex grow.
    #[must_use]
    pub const fn grow(mut self, grow: f32) -> Self {
        self.0.flex_grow = grow;
        self
    }

    /// Set flex shrink.
    #[must_use]
    pub const fn shrink(mut self, shrink: f32) -> Self {
        self.0.flex_shrink = shrink;
        self
    }

    /// Set cross-axis child alignment for flex/grid containers.
    #[must_use]
    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.0.align_items = Some(align_items);
        self
    }

    /// Set main-axis content distribution for flex/grid containers.
    #[must_use]
    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.0.justify_content = Some(justify_content);
        self
    }

    /// Set uniform margin on all four sides.
    #[must_use]
    pub const fn margin(mut self, margin: LengthPercentageAuto) -> Self {
        self.0.margin = all_sides(margin);
        self
    }

    /// Set a fixed logical-pixel uniform margin.
    #[must_use]
    pub const fn margin_px(self, margin: f32) -> Self {
        self.margin(LengthPercentageAuto::length(margin))
    }

    /// Set the minimum width and height.
    #[must_use]
    pub const fn min_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.0.min_size = TaffySize { width, height };
        self
    }

    /// Set a fixed logical-pixel minimum width and height.
    #[must_use]
    pub const fn min_size_px(self, width: f32, height: f32) -> Self {
        self.min_size(Dimension::length(width), Dimension::length(height))
    }

    /// Set the maximum width and height.
    #[must_use]
    pub const fn max_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.0.max_size = TaffySize { width, height };
        self
    }

    /// Set a fixed logical-pixel maximum width and height.
    #[must_use]
    pub const fn max_size_px(self, width: f32, height: f32) -> Self {
        self.max_size(Dimension::length(width), Dimension::length(height))
    }

    /// Override the cross-axis alignment for this item specifically.
    ///
    /// Overrides the parent container's `align_items` for this child only.
    #[must_use]
    pub const fn align_self(mut self, align_self: AlignSelf) -> Self {
        self.0.align_self = Some(align_self);
        self
    }

    /// Set the initial flex item size before `grow`/`shrink` are applied.
    #[must_use]
    pub const fn flex_basis(mut self, basis: Dimension) -> Self {
        self.0.flex_basis = basis;
        self
    }

    /// Set the position mode (`Relative` or `Absolute`).
    ///
    /// Use `Position::Absolute` with `inset` (via [`with_taffy`](Self::with_taffy))
    /// to place a node at an exact offset from its containing block.
    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self
    }

    /// Escape hatch for raw `taffy::Style` mutation.
    ///
    /// Use named methods when possible. Reach for this only when you need
    /// a Taffy field that has no dedicated builder method yet.
    #[must_use]
    pub fn with_taffy(mut self, mutate: impl FnOnce(&mut Style)) -> Self {
        mutate(&mut self.0);
        self
    }

    pub(crate) const fn as_taffy(&self) -> &Style {
        &self.0
    }
}

const fn all_sides<T: Copy>(value: T) -> TaffyRect<T> {
    TaffyRect {
        left: value,
        right: value,
        top: value,
        bottom: value,
    }
}
