use crate::layout::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Overflow, Position, Rect, Size,
    StyleData,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutStyle(pub(crate) StyleData);

impl LayoutStyle {
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

    #[must_use]
    pub fn flex_row() -> Self {
        Self(StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        })
    }

    #[must_use]
    pub fn flex_column() -> Self {
        Self(StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        })
    }

    #[must_use]
    pub fn grid(columns: Vec<crate::layout::TrackSize>) -> Self {
        Self(StyleData {
            display: Display::Grid,
            box_sizing: BoxSizing::BorderBox,
            grid_template_columns: columns,
            ..StyleData::default()
        })
    }

    #[must_use]
    pub fn grid_columns(mut self, columns: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_columns = columns;
        self
    }

    #[must_use]
    pub fn grid_rows(mut self, rows: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_rows = rows;
        self
    }

    #[must_use]
    pub const fn grid_column_span(mut self, span: u16) -> Self {
        self.0.grid_column_span = if span > 0 { span } else { 1 };
        self
    }

    #[must_use]
    pub const fn grid_row_span(mut self, span: u16) -> Self {
        self.0.grid_row_span = if span > 0 { span } else { 1 };
        self
    }

    #[must_use]
    pub const fn grid_area(mut self, col: u16, row: u16) -> Self {
        self.0.grid_column_start = col;
        self.0.grid_row_start = row;
        self
    }

    #[must_use]
    pub const fn grid_dense(mut self) -> Self {
        self.0.grid_auto_flow_dense = true;
        self
    }

    #[must_use]
    pub const fn display(mut self, display: Display) -> Self {
        self.0.display = display;
        self
    }

    #[must_use]
    pub const fn flex_direction(mut self, flex_direction: FlexDirection) -> Self {
        self.0.flex_direction = flex_direction;
        self
    }

    #[must_use]
    pub const fn flex_wrap(mut self, flex_wrap: FlexWrap) -> Self {
        self.0.flex_wrap = flex_wrap;
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Dimension>) -> Self {
        self.0.size.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Dimension>) -> Self {
        self.0.size.height = height.into();
        self
    }

    #[must_use]
    pub fn min_width(mut self, value: impl Into<Dimension>) -> Self {
        self.0.min_size.width = value.into();
        self
    }

    #[must_use]
    pub fn min_height(mut self, value: impl Into<Dimension>) -> Self {
        self.0.min_size.height = value.into();
        self
    }

    #[must_use]
    pub fn max_width(mut self, value: impl Into<Dimension>) -> Self {
        self.0.max_size.width = value.into();
        self
    }

    #[must_use]
    pub fn max_height(mut self, value: impl Into<Dimension>) -> Self {
        self.0.max_size.height = value.into();
        self
    }

    #[must_use]
    pub const fn fixed_size(mut self, width: f32, height: f32) -> Self {
        self.0.size.width = Dimension::Px(width);
        self.0.size.height = Dimension::Px(height);
        self
    }

    #[must_use]
    pub fn pane(width: f32, height: f32) -> Self {
        Self::flex_column().fixed_size(width, height).gap(4.0)
    }

    #[must_use]
    pub const fn fill_width(mut self) -> Self {
        self.0.size.width = Dimension::Fill;
        self
    }

    #[must_use]
    pub const fn fill_height(mut self) -> Self {
        self.0.size.height = Dimension::Fill;
        self
    }

    #[must_use]
    pub const fn fill(mut self) -> Self {
        self.0.size.width = Dimension::Fill;
        self.0.size.height = Dimension::Fill;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.0.padding = all_sides(padding.into());
        self
    }

    #[must_use]
    pub fn padding_x(mut self, padding: impl Into<LengthPercentage>) -> Self {
        let padding = padding.into();
        self.0.padding.left = padding;
        self.0.padding.right = padding;
        self
    }

    #[must_use]
    pub fn padding_y(mut self, padding: impl Into<LengthPercentage>) -> Self {
        let padding = padding.into();
        self.0.padding.top = padding;
        self.0.padding.bottom = padding;
        self
    }

    #[must_use]
    pub fn padding_top(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.0.padding.top = padding.into();
        self
    }

    #[must_use]
    pub fn padding_right(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.0.padding.right = padding.into();
        self
    }

    #[must_use]
    pub fn padding_bottom(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.0.padding.bottom = padding.into();
        self
    }

    #[must_use]
    pub fn padding_left(mut self, padding: impl Into<LengthPercentage>) -> Self {
        self.0.padding.left = padding.into();
        self
    }

    #[must_use]
    pub const fn overflow_x(mut self, overflow: Overflow) -> Self {
        self.0.overflow.width = overflow;
        self
    }

    #[must_use]
    pub const fn overflow_y(mut self, overflow: Overflow) -> Self {
        self.0.overflow.height = overflow;
        self
    }

    #[must_use]
    pub const fn overflow(self, overflow: Overflow) -> Self {
        self.overflow_x(overflow).overflow_y(overflow)
    }

    #[must_use]
    pub const fn clips_paint(&self) -> bool {
        !matches!(self.0.overflow.height, Overflow::Visible)
            || !matches!(self.0.overflow.width, Overflow::Visible)
    }

    #[must_use]
    pub fn inset_top(mut self, top: impl Into<LengthPercentageAuto>) -> Self {
        self.0.inset.top = top.into();
        self
    }

    #[must_use]
    pub fn inset_left(mut self, left: impl Into<LengthPercentageAuto>) -> Self {
        self.0.inset.left = left.into();
        self
    }

    #[must_use]
    pub fn inset_right(mut self, right: impl Into<LengthPercentageAuto>) -> Self {
        self.0.inset.right = right.into();
        self
    }

    #[must_use]
    pub fn inset_bottom(mut self, bottom: impl Into<LengthPercentageAuto>) -> Self {
        self.0.inset.bottom = bottom.into();
        self
    }

    #[must_use]
    pub fn inset(mut self, inset: impl Into<LengthPercentageAuto>) -> Self {
        self.0.inset = all_sides(inset.into());
        self
    }

    #[must_use]
    pub fn gap(mut self, gap: impl Into<LengthPercentage>) -> Self {
        let gap = gap.into();
        self.0.gap = Size {
            width: gap,
            height: gap,
        };
        self
    }

    #[must_use]
    pub const fn grow(mut self, grow: f32) -> Self {
        self.0.flex_grow = grow;
        if matches!(self.0.min_size.width, Dimension::Auto) {
            self.0.min_size.width = Dimension::Px(0.0);
        }
        if matches!(self.0.min_size.height, Dimension::Auto) {
            self.0.min_size.height = Dimension::Px(0.0);
        }
        self
    }

    #[must_use]
    pub const fn shrink(mut self, shrink: f32) -> Self {
        self.0.flex_shrink = shrink;
        self
    }

    #[must_use]
    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.0.align_items = Some(align_items);
        self
    }

    #[must_use]
    pub const fn align_content(mut self, align_content: AlignContent) -> Self {
        self.0.align_content = Some(align_content);
        self
    }

    #[must_use]
    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.0.justify_content = Some(justify_content);
        self
    }

    #[must_use]
    pub fn margin(mut self, margin: impl Into<LengthPercentageAuto>) -> Self {
        self.0.margin = all_sides(margin.into());
        self
    }

    #[must_use]
    pub fn min_size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
        self.0.min_size = Size {
            width: width.into(),
            height: height.into(),
        };
        self
    }

    #[must_use]
    pub fn max_size(mut self, width: impl Into<Dimension>, height: impl Into<Dimension>) -> Self {
        self.0.max_size = Size {
            width: width.into(),
            height: height.into(),
        };
        self
    }

    #[must_use]
    pub const fn align_self(mut self, align_self: AlignSelf) -> Self {
        self.0.align_self = Some(align_self);
        self
    }

    #[must_use]
    pub fn flex_basis(mut self, basis: impl Into<Dimension>) -> Self {
        self.0.flex_basis = basis.into();
        self
    }

    #[must_use]
    pub const fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        self.0.box_sizing = box_sizing;
        self
    }

    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self
    }

    #[must_use]
    pub fn with_layout(mut self, mutate: impl FnOnce(&mut StyleData)) -> Self {
        mutate(&mut self.0);
        self
    }

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

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
