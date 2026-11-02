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
#[derive(Clone, Debug, Default)]
pub struct LayoutStyle(pub(crate) StyleData, LayoutFields);

impl PartialEq for LayoutStyle {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl LayoutStyle {
    /// Returns the block layout style.
    #[must_use]
    pub fn block() -> Self {
        let style = StyleData {
            display: Display::Block,
            box_sizing: BoxSizing::BorderBox,
            flex_direction: FlexDirection::Column,
            ..StyleData::default()
        };
        Self(style, BOX.with(LayoutFields::DIRECTION))
    }

    /// Returns the flex row layout style.
    #[must_use]
    pub fn flex_row() -> Self {
        Self::flex(FlexDirection::Row)
    }

    /// Returns the flex column layout style.
    #[must_use]
    pub fn flex_column() -> Self {
        Self::flex(FlexDirection::Column)
    }

    fn flex(flex_direction: FlexDirection) -> Self {
        let style = StyleData {
            display: Display::Flex,
            flex_direction,
            box_sizing: BoxSizing::BorderBox,
            ..StyleData::default()
        };
        Self(style, BOX.with(LayoutFields::DIRECTION))
    }

    /// Creates a grid style with these column tracks.
    #[must_use]
    pub fn grid(columns: Vec<crate::layout::TrackSize>) -> Self {
        let style = StyleData {
            display: Display::Grid,
            box_sizing: BoxSizing::BorderBox,
            grid_template_columns: columns,
            ..StyleData::default()
        };
        Self(style, BOX.with(LayoutFields::GRID_COLUMNS))
    }

    /// Sets the grid columns.
    #[must_use]
    pub fn grid_columns(mut self, columns: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_columns = columns;
        self.mark(LayoutFields::GRID_COLUMNS)
    }

    /// Sets the grid rows.
    #[must_use]
    pub fn grid_rows(mut self, rows: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_rows = rows;
        self.mark(LayoutFields::GRID_ROWS)
    }

    /// Sets the grid column span.
    #[must_use]
    pub const fn grid_column_span(mut self, span: u16) -> Self {
        self.0.grid_column_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::COLUMN_SPAN)
    }

    /// Sets the grid row span.
    #[must_use]
    pub const fn grid_row_span(mut self, span: u16) -> Self {
        self.0.grid_row_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::ROW_SPAN)
    }

    /// Sets the grid area.
    #[must_use]
    pub const fn grid_area(mut self, col: u16, row: u16) -> Self {
        self.0.grid_column_start = col;
        self.0.grid_row_start = row;
        self.mark(LayoutFields::COLUMN_START.with(LayoutFields::ROW_START))
    }

    /// Applies the grid dense option.
    #[must_use]
    pub const fn grid_dense(mut self) -> Self {
        self.0.grid_auto_flow_dense = true;
        self.mark(LayoutFields::DENSE)
    }

    /// Sets the display.
    #[must_use]
    pub const fn display(mut self, display: Display) -> Self {
        self.0.display = display;
        self.mark(LayoutFields::DISPLAY)
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Sets the flex direction.
    #[must_use]
    #[doc(hidden)]
    pub const fn flex_direction(mut self, flex_direction: FlexDirection) -> Self {
        self.0.flex_direction = flex_direction;
        self.mark(LayoutFields::DIRECTION)
    }

    /// Sets the flex wrap.
    #[must_use]
    pub const fn flex_wrap(mut self, flex_wrap: FlexWrap) -> Self {
        self.0.flex_wrap = flex_wrap;
        self.mark(LayoutFields::WRAP)
    }

    /// Sets the align items.
    #[must_use]
    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.0.align_items = Some(align_items);
        self.mark(LayoutFields::ALIGN_ITEMS)
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Sets the align content.
    #[must_use]
    #[doc(hidden)]
    pub const fn align_content(mut self, align_content: AlignContent) -> Self {
        self.0.align_content = Some(align_content);
        self.mark(LayoutFields::ALIGN_CONTENT)
    }

    /// Sets the justify content.
    #[must_use]
    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.0.justify_content = Some(justify_content);
        self.mark(LayoutFields::JUSTIFY)
    }

    /// Sets the align self.
    #[must_use]
    pub const fn align_self(mut self, align_self: AlignItems) -> Self {
        self.0.align_self = Some(align_self);
        self.mark(LayoutFields::ALIGN_SELF)
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Sets the box sizing.
    #[must_use]
    #[doc(hidden)]
    pub const fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        self.0.box_sizing = box_sizing;
        self.mark(LayoutFields::BOX_SIZING)
    }

    /// Sets the position.
    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self.mark(LayoutFields::POSITION)
    }

    #[cfg(any(test, feature = "kittest"))]
    #[doc(hidden)]
    #[must_use]
    pub fn with_layout(mut self, mutate: impl FnOnce(&mut StyleData)) -> Self {
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

#[cfg(test)]
#[path = "intent_tests.rs"]
mod tests;
