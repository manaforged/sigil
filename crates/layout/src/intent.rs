//! Layout intent attached to `sigil-kernel` nodes.

use crate::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Overflow, Position, Rect, Size,
    StyleData,
};

/// Layout intent attached to a node through `NodeExtensions`.
///
/// This is a thin newtype over owned [`StyleData`]; keeping the wrapper
/// at the public boundary preserves the adapter as the stable authoring
/// surface while the layout backend evolves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutStyle(pub(crate) StyleData);

impl LayoutStyle {
    /// Build a block layout intent.
    #[must_use]
    pub fn block() -> Self {
        let mut style = StyleData {
            display: Display::Block,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        };
        style.flex_direction = FlexDirection::Column;
        Self(style)
    }

    /// Build a row-oriented flex layout intent.
    #[must_use]
    pub fn flex_row() -> Self {
        Self(StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        })
    }

    /// Build a column-oriented flex layout intent.
    #[must_use]
    pub fn flex_column() -> Self {
        Self(StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        })
    }

    /// Build a grid layout intent.
    #[must_use]
    pub fn grid() -> Self {
        Self(StyleData {
            display: Display::Grid,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        })
    }

    /// Set the layout algorithm selector.
    #[must_use]
    pub const fn display(mut self, display: Display) -> Self {
        self.0.display = display;
        self
    }

    /// Set the flex main-axis direction.
    #[must_use]
    pub const fn flex_direction(mut self, flex_direction: FlexDirection) -> Self {
        self.0.flex_direction = flex_direction;
        self
    }

    /// Set the flex line-wrap mode.
    #[must_use]
    pub const fn flex_wrap(mut self, flex_wrap: FlexWrap) -> Self {
        self.0.flex_wrap = flex_wrap;
        self
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
        self.width(Dimension::Px(width))
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
        self.height(Dimension::Px(height))
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
        self.padding(LengthPercentage::Px(padding))
    }

    /// Set fixed logical-pixel padding on the inline (left + right) axis.
    #[must_use]
    pub const fn padding_x_px(mut self, padding: f32) -> Self {
        self.0.padding.left = LengthPercentage::Px(padding);
        self.0.padding.right = LengthPercentage::Px(padding);
        self
    }

    /// Set fixed logical-pixel padding on the block (top + bottom) axis.
    #[must_use]
    pub const fn padding_y_px(mut self, padding: f32) -> Self {
        self.0.padding.top = LengthPercentage::Px(padding);
        self.0.padding.bottom = LengthPercentage::Px(padding);
        self
    }

    /// Set the block-axis (height) overflow behavior.
    #[must_use]
    pub const fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.0.overflow.height = overflow;
        self
    }

    /// Set a fixed logical-pixel top inset (for `Position::Absolute`).
    #[must_use]
    pub const fn inset_top_px(mut self, top: f32) -> Self {
        self.0.inset.top = LengthPercentageAuto::Px(top);
        self
    }

    /// Set a fixed logical-pixel left inset (for `Position::Absolute`).
    #[must_use]
    pub const fn inset_left_px(mut self, left: f32) -> Self {
        self.0.inset.left = LengthPercentageAuto::Px(left);
        self
    }

    /// Set uniform flex/grid gap.
    #[must_use]
    pub const fn gap(mut self, gap: LengthPercentage) -> Self {
        self.0.gap = Size {
            width: gap,
            height: gap,
        };
        self
    }

    /// Set a fixed logical-pixel flex/grid gap.
    #[must_use]
    pub const fn gap_px(self, gap: f32) -> Self {
        self.gap(LengthPercentage::Px(gap))
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

    /// Set per-line cross-axis distribution for flex/grid containers.
    #[must_use]
    pub const fn align_content(mut self, align_content: AlignContent) -> Self {
        self.0.align_content = Some(align_content);
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
        self.margin(LengthPercentageAuto::Px(margin))
    }

    /// Set the minimum width and height.
    #[must_use]
    pub const fn min_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.0.min_size = Size { width, height };
        self
    }

    /// Set a fixed logical-pixel minimum width and height.
    #[must_use]
    pub const fn min_size_px(self, width: f32, height: f32) -> Self {
        self.min_size(Dimension::Px(width), Dimension::Px(height))
    }

    /// Set the maximum width and height.
    #[must_use]
    pub const fn max_size(mut self, width: Dimension, height: Dimension) -> Self {
        self.0.max_size = Size { width, height };
        self
    }

    /// Set a fixed logical-pixel maximum width and height.
    #[must_use]
    pub const fn max_size_px(self, width: f32, height: f32) -> Self {
        self.max_size(Dimension::Px(width), Dimension::Px(height))
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

    /// Set whether `size` includes padding and border.
    #[must_use]
    pub const fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        self.0.box_sizing = box_sizing;
        self
    }

    /// Set the position mode (`Relative` or `Absolute`).
    ///
    /// Use `Position::Absolute` with `inset` (via [`with_layout`](Self::with_layout))
    /// to place a node at an exact offset from its containing block.
    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self
    }

    /// Escape hatch for direct [`StyleData`] mutation.
    ///
    /// Use named methods when possible. Reach for this only when you need
    /// a layout field that has no dedicated builder method yet.
    #[must_use]
    pub fn with_layout(mut self, mutate: impl FnOnce(&mut StyleData)) -> Self {
        mutate(&mut self.0);
        self
    }

    /// Borrow the underlying resolved [`StyleData`].
    ///
    /// Public so tests and other crates can verify what a builder chain
    /// (or a slice of [`StyleAtom`]s via
    /// [`crate::style_adapter::apply_style_atoms`]) resolved to.
    #[must_use]
    pub const fn data(&self) -> &StyleData {
        &self.0
    }

    pub(crate) const fn data_mut(&mut self) -> &mut StyleData {
        &mut self.0
    }
}

const fn all_sides<T: Copy>(value: T) -> Rect<T> {
    Rect::splat(value)
}
