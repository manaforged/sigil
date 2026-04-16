//! Layout intent attached to `sigil-core` nodes.

use taffy::geometry::{Rect as TaffyRect, Size as TaffySize};
use taffy::prelude::{
    AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage, Style,
};

/// Layout intent attached to a node through `NodeExtensions`.
///
/// This is a thin newtype over `taffy::Style`; keeping the wrapper at
/// the public boundary preserves the adapter as the swap point if
/// Taffy churns or a measured fast path appears later.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutIntent(pub(crate) Style);

impl LayoutIntent {
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

    /// Apply a focused raw Taffy style mutation.
    #[must_use]
    pub fn with_style(mut self, mutate: impl FnOnce(&mut Style)) -> Self {
        mutate(&mut self.0);
        self
    }

    /// Borrow the wrapped Taffy style.
    #[must_use]
    pub const fn style(&self) -> &Style {
        &self.0
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
