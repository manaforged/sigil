use crate::layout::style_data::{Dimension, Size};

use super::{LayoutFields, LayoutStyle};

impl LayoutStyle {
    #[must_use]
    pub fn width(mut self, width: impl Into<Dimension>) -> Self {
        self.0.size.width = width.into();
        self.mark(LayoutFields::WIDTH)
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Dimension>) -> Self {
        self.0.size.height = height.into();
        self.mark(LayoutFields::HEIGHT)
    }

    #[must_use]
    pub fn min_width(mut self, value: impl Into<Dimension>) -> Self {
        self.0.min_size.width = value.into();
        self.mark(LayoutFields::MIN_WIDTH)
    }

    #[must_use]
    pub fn min_height(mut self, value: impl Into<Dimension>) -> Self {
        self.0.min_size.height = value.into();
        self.mark(LayoutFields::MIN_HEIGHT)
    }

    #[must_use]
    pub fn max_width(mut self, value: impl Into<Dimension>) -> Self {
        self.0.max_size.width = value.into();
        self.mark(LayoutFields::MAX_WIDTH)
    }

    #[must_use]
    pub fn max_height(mut self, value: impl Into<Dimension>) -> Self {
        self.0.max_size.height = value.into();
        self.mark(LayoutFields::MAX_HEIGHT)
    }

    #[must_use]
    pub const fn fixed_size(mut self, width: f32, height: f32) -> Self {
        self.0.size.width = Dimension::Px(width);
        self.0.size.height = Dimension::Px(height);
        self.mark(LayoutFields::WIDTH.with(LayoutFields::HEIGHT))
    }

    #[must_use]
    pub fn pane(width: f32, height: f32) -> Self {
        Self::flex_column().fixed_size(width, height).gap(4.0)
    }

    #[must_use]
    pub const fn fill_width(mut self) -> Self {
        self.0.size.width = Dimension::Fill;
        self.mark(LayoutFields::WIDTH)
    }

    #[must_use]
    pub const fn fill_height(mut self) -> Self {
        self.0.size.height = Dimension::Fill;
        self.mark(LayoutFields::HEIGHT)
    }

    #[must_use]
    pub const fn fill(mut self) -> Self {
        self.0.size.width = Dimension::Fill;
        self.0.size.height = Dimension::Fill;
        self.mark(LayoutFields::WIDTH.with(LayoutFields::HEIGHT))
    }

    #[must_use]
    pub const fn grow(mut self, grow: f32) -> Self {
        self.0.flex_grow = grow;
        let mut set = LayoutFields::GROW;
        if matches!(self.0.min_size.width, Dimension::Auto) {
            self.0.min_size.width = Dimension::Px(0.0);
            set = set.with(LayoutFields::MIN_WIDTH);
        }
        if matches!(self.0.min_size.height, Dimension::Auto) {
            self.0.min_size.height = Dimension::Px(0.0);
            set = set.with(LayoutFields::MIN_HEIGHT);
        }
        self.mark(set)
    }

    #[must_use]
    pub const fn shrink(mut self, shrink: f32) -> Self {
        self.0.flex_shrink = shrink;
        self.mark(LayoutFields::SHRINK)
    }

    #[must_use]
    pub fn min_size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
        self.0.min_size = Size {
            width: width.into(),
            height: height.into(),
        };
        self.mark(LayoutFields::MIN_WIDTH.with(LayoutFields::MIN_HEIGHT))
    }

    #[must_use]
    pub fn max_size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
        self.0.max_size = Size {
            width: width.into(),
            height: height.into(),
        };
        self.mark(LayoutFields::MAX_WIDTH.with(LayoutFields::MAX_HEIGHT))
    }

    #[must_use]
    pub fn flex_basis(mut self, basis: impl Into<Dimension>) -> Self {
        self.0.flex_basis = basis.into();
        self.mark(LayoutFields::BASIS)
    }
}
