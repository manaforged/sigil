use crate::layout::style_data::{Axes, Length};

use super::{LayoutFields, LayoutStyle};

impl LayoutStyle {
    /// Sets the width of the node.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.0.size.width = width.into();
        self.mark(LayoutFields::WIDTH)
    }

    /// Sets the height of the node.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.0.size.height = height.into();
        self.mark(LayoutFields::HEIGHT)
    }

    /// Sets the smallest width that the node can have.
    pub fn min_width(mut self, value: impl Into<Length>) -> Self {
        self.0.min_size.width = value.into();
        self.mark(LayoutFields::MIN_WIDTH)
    }

    /// Sets the smallest height that the node can have.
    pub fn min_height(mut self, value: impl Into<Length>) -> Self {
        self.0.min_size.height = value.into();
        self.mark(LayoutFields::MIN_HEIGHT)
    }

    /// Sets the largest width that the node can have.
    pub fn max_width(mut self, value: impl Into<Length>) -> Self {
        self.0.max_size.width = value.into();
        self.mark(LayoutFields::MAX_WIDTH)
    }

    /// Sets the largest height that the node can have.
    pub fn max_height(mut self, value: impl Into<Length>) -> Self {
        self.0.max_size.height = value.into();
        self.mark(LayoutFields::MAX_HEIGHT)
    }

    /// Sets the width and the height in pixels.
    pub const fn fixed_size(mut self, width: f32, height: f32) -> Self {
        self.0.size = Axes::new(Length::Px(width), Length::Px(height));
        self.mark(LayoutFields::WIDTH.with(LayoutFields::HEIGHT))
    }

    /// Sets the width to `Length::Fill`.
    pub const fn fill_width(mut self) -> Self {
        self.0.size.width = Length::Fill;
        self.mark(LayoutFields::WIDTH)
    }

    /// Sets the height to `Length::Fill`.
    pub const fn fill_height(mut self) -> Self {
        self.0.size.height = Length::Fill;
        self.mark(LayoutFields::HEIGHT)
    }

    /// Sets the width and the height to `Length::Fill`.
    pub const fn fill(mut self) -> Self {
        self.0.size = Axes::splat(Length::Fill);
        self.mark(LayoutFields::WIDTH.with(LayoutFields::HEIGHT))
    }

    /// Sets the flex grow factor; an automatic minimum width or height becomes 0 px.
    pub const fn grow(mut self, grow: f32) -> Self {
        self.0.flex_grow = grow;
        let mut set = LayoutFields::GROW;
        if matches!(self.0.min_size.width, Length::Auto) {
            self.0.min_size.width = Length::Px(0.0);
            set = set.with(LayoutFields::MIN_WIDTH);
        }
        if matches!(self.0.min_size.height, Length::Auto) {
            self.0.min_size.height = Length::Px(0.0);
            set = set.with(LayoutFields::MIN_HEIGHT);
        }
        self.mark(set)
    }

    /// Sets the flex shrink factor.
    pub const fn shrink(mut self, shrink: f32) -> Self {
        self.0.flex_shrink = shrink;
        self.mark(LayoutFields::SHRINK)
    }

    pub(crate) const fn stretch_width(mut self) -> Self {
        self.0.stretch_width = true;
        self.mark(LayoutFields::STRETCH_WIDTH)
    }

    /// Sets the smallest width and height that the node can have.
    pub fn min_size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.0.min_size = Axes::new(width.into(), height.into());
        self.mark(LayoutFields::MIN_WIDTH.with(LayoutFields::MIN_HEIGHT))
    }

    /// Sets the largest width and height that the node can have.
    pub fn max_size(mut self, width: impl Into<Length>, height: impl Into<Length>) -> Self {
        self.0.max_size = Axes::new(width.into(), height.into());
        self.mark(LayoutFields::MAX_WIDTH.with(LayoutFields::MAX_HEIGHT))
    }

    /// Sets the width-to-height ratio that the node keeps when one side is set and the other is auto.
    pub const fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.0.aspect_ratio = Some(ratio);
        self.mark(LayoutFields::ASPECT_RATIO)
    }

    /// Sets the flex basis of the node.
    pub fn flex_basis(mut self, basis: impl Into<Length>) -> Self {
        self.0.flex_basis = basis.into();
        self.mark(LayoutFields::BASIS)
    }
}
