use crate::layout::style_data::{
    AlignContent, AlignItems, BoxSizing, Display, FlexDirection, FlexWrap, JustifyContent,
    Position, StyleData,
};
use crate::style::merge::SlotSet;

mod fields;
mod size;
mod spacing;

pub(crate) use fields::LayoutFields;

/// Layout description of a node.
#[derive(Clone, Debug)]
pub struct LayoutStyle(pub(crate) StyleData, LayoutFields);

impl Default for LayoutStyle {
    fn default() -> Self {
        Self(border_box(), LayoutFields::default())
    }
}

impl PartialEq for LayoutStyle {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl LayoutStyle {
    /// Creates a block style with a column direction.
    #[must_use]
    pub fn block() -> Self {
        let style = StyleData {
            display: Display::Block,
            flex_direction: FlexDirection::Column,
            ..border_box()
        };
        Self(style, BOX.with(LayoutFields::DIRECTION))
    }

    /// Creates a flex style with a row direction.
    #[must_use]
    pub fn flex_row() -> Self {
        Self::flex(FlexDirection::Row)
    }

    /// Creates a flex style with a column direction.
    #[must_use]
    pub fn flex_column() -> Self {
        Self::flex(FlexDirection::Column)
    }

    fn flex(flex_direction: FlexDirection) -> Self {
        let style = StyleData {
            display: Display::Flex,
            flex_direction,
            ..border_box()
        };
        Self(style, BOX.with(LayoutFields::DIRECTION))
    }

    /// Creates a grid style with `columns` as its column tracks.
    #[must_use]
    pub fn grid(columns: Vec<crate::layout::TrackSize>) -> Self {
        let style = StyleData {
            display: Display::Grid,
            grid_template_columns: columns,
            ..border_box()
        };
        Self(style, BOX.with(LayoutFields::GRID_COLUMNS))
    }

    /// Sets the column tracks of a grid.
    #[must_use]
    pub fn grid_columns(mut self, columns: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_columns = columns;
        self.mark(LayoutFields::GRID_COLUMNS)
    }

    /// Sets the row tracks of a grid.
    #[must_use]
    pub fn grid_rows(mut self, rows: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_rows = rows;
        self.mark(LayoutFields::GRID_ROWS)
    }

    /// Sets the number of grid columns that the node spans; 0 counts as 1.
    #[must_use]
    pub const fn grid_column_span(mut self, span: u16) -> Self {
        self.0.grid_column_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::COLUMN_SPAN)
    }

    /// Sets the number of grid rows that the node spans; 0 counts as 1.
    #[must_use]
    pub const fn grid_row_span(mut self, span: u16) -> Self {
        self.0.grid_row_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::ROW_SPAN)
    }

    /// Sets the grid cell of the node by column and row, counted from 1; 0 means no fixed cell.
    #[must_use]
    pub const fn grid_area(mut self, col: u16, row: u16) -> Self {
        self.0.grid_column_start = col;
        self.0.grid_row_start = row;
        self.mark(LayoutFields::COLUMN_START.with(LayoutFields::ROW_START))
    }

    /// Turns on dense auto flow for the grid.
    #[must_use]
    pub const fn grid_dense(mut self) -> Self {
        self.0.grid_auto_flow_dense = true;
        self.mark(LayoutFields::DENSE)
    }

    /// Sets the display mode: block, flex, grid, or none.
    #[must_use]
    pub const fn display(mut self, display: Display) -> Self {
        self.0.display = display;
        self.mark(LayoutFields::DISPLAY)
    }

    #[cfg(feature = "kittest")]
    /// Sets the main axis of a flex box.
    #[must_use]
    #[doc(hidden)]
    pub const fn flex_direction(mut self, flex_direction: FlexDirection) -> Self {
        self.0.flex_direction = flex_direction;
        self.mark(LayoutFields::DIRECTION)
    }

    /// Sets the wrap mode of a flex box.
    #[must_use]
    pub const fn flex_wrap(mut self, flex_wrap: FlexWrap) -> Self {
        self.0.flex_wrap = flex_wrap;
        self.mark(LayoutFields::WRAP)
    }

    /// Sets the `align-items` value for the children of the node.
    #[must_use]
    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.0.align_items = Some(align_items);
        self.mark(LayoutFields::ALIGN_ITEMS)
    }

    #[cfg(feature = "kittest")]
    /// Sets the `align-content` value of the node.
    #[must_use]
    #[doc(hidden)]
    pub const fn align_content(mut self, align_content: AlignContent) -> Self {
        self.0.align_content = Some(align_content);
        self.mark(LayoutFields::ALIGN_CONTENT)
    }

    /// Sets the `justify-content` value of the node.
    #[must_use]
    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.0.justify_content = Some(justify_content);
        self.mark(LayoutFields::JUSTIFY)
    }

    /// Sets the `align-self` value of the node.
    #[must_use]
    pub const fn align_self(mut self, align_self: AlignItems) -> Self {
        self.0.align_self = Some(align_self);
        self.mark(LayoutFields::ALIGN_SELF)
    }

    #[cfg(feature = "kittest")]
    /// Sets the box sizing mode of the node.
    #[must_use]
    #[doc(hidden)]
    pub const fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        self.0.box_sizing = box_sizing;
        self.mark(LayoutFields::BOX_SIZING)
    }

    /// Sets the position mode: `Position::Relative` or `Position::Absolute`.
    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self.mark(LayoutFields::POSITION)
    }

    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub fn edit(mut self, mutate: impl FnOnce(&mut StyleData)) -> Self {
        mutate(&mut self.0);
        self.mark(LayoutFields::ALL)
    }

    #[must_use]
    pub(crate) const fn data(&self) -> &StyleData {
        &self.0
    }

    pub(crate) const fn data_mut(&mut self) -> &mut StyleData {
        &mut self.0
    }

    pub(crate) fn into_data(self) -> StyleData {
        self.0
    }

    pub(crate) fn merge(&mut self, other: &Self) {
        fields::merge(&mut self.0, &other.0, other.1);
        self.1 = self.1.with(other.1);
    }

    pub(crate) fn slots(&self) -> SlotSet {
        fields::slots(self.1)
    }

    const fn mark(mut self, set: LayoutFields) -> Self {
        self.1 = self.1.with(set);
        self
    }
}

const BOX: LayoutFields = LayoutFields::DISPLAY.with(LayoutFields::BOX_SIZING);

fn border_box() -> StyleData {
    StyleData {
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    }
}

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
