use crate::layout::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Display, FlexDirection, FlexWrap,
    JustifyContent, Position, StyleData,
};
use crate::style::merge::SlotSet;

mod fields;
mod size;
mod spacing;

pub(crate) use fields::LayoutFields;

#[derive(Clone, Debug, Default)]
pub struct LayoutStyle(pub(crate) StyleData, LayoutFields);

impl PartialEq for LayoutStyle {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl LayoutStyle {
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

    #[must_use]
    pub fn flex_row() -> Self {
        Self::flex(FlexDirection::Row)
    }

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

    #[must_use]
    pub fn grid_columns(mut self, columns: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_columns = columns;
        self.mark(LayoutFields::GRID_COLUMNS)
    }

    #[must_use]
    pub fn grid_rows(mut self, rows: Vec<crate::layout::TrackSize>) -> Self {
        self.0.grid_template_rows = rows;
        self.mark(LayoutFields::GRID_ROWS)
    }

    #[must_use]
    pub const fn grid_column_span(mut self, span: u16) -> Self {
        self.0.grid_column_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::COLUMN_SPAN)
    }

    #[must_use]
    pub const fn grid_row_span(mut self, span: u16) -> Self {
        self.0.grid_row_span = if span > 0 { span } else { 1 };
        self.mark(LayoutFields::ROW_SPAN)
    }

    #[must_use]
    pub const fn grid_area(mut self, col: u16, row: u16) -> Self {
        self.0.grid_column_start = col;
        self.0.grid_row_start = row;
        self.mark(LayoutFields::COLUMN_START.with(LayoutFields::ROW_START))
    }

    #[must_use]
    pub const fn grid_dense(mut self) -> Self {
        self.0.grid_auto_flow_dense = true;
        self.mark(LayoutFields::DENSE)
    }

    #[must_use]
    pub const fn display(mut self, display: Display) -> Self {
        self.0.display = display;
        self.mark(LayoutFields::DISPLAY)
    }

    #[must_use]
    pub const fn flex_direction(mut self, flex_direction: FlexDirection) -> Self {
        self.0.flex_direction = flex_direction;
        self.mark(LayoutFields::DIRECTION)
    }

    #[must_use]
    pub const fn flex_wrap(mut self, flex_wrap: FlexWrap) -> Self {
        self.0.flex_wrap = flex_wrap;
        self.mark(LayoutFields::WRAP)
    }

    #[must_use]
    pub const fn align_items(mut self, align_items: AlignItems) -> Self {
        self.0.align_items = Some(align_items);
        self.mark(LayoutFields::ALIGN_ITEMS)
    }

    #[must_use]
    pub const fn align_content(mut self, align_content: AlignContent) -> Self {
        self.0.align_content = Some(align_content);
        self.mark(LayoutFields::ALIGN_CONTENT)
    }

    #[must_use]
    pub const fn justify_content(mut self, justify_content: JustifyContent) -> Self {
        self.0.justify_content = Some(justify_content);
        self.mark(LayoutFields::JUSTIFY)
    }

    #[must_use]
    pub const fn align_self(mut self, align_self: AlignSelf) -> Self {
        self.0.align_self = Some(align_self);
        self.mark(LayoutFields::ALIGN_SELF)
    }

    #[must_use]
    pub const fn box_sizing(mut self, box_sizing: BoxSizing) -> Self {
        self.0.box_sizing = box_sizing;
        self.mark(LayoutFields::BOX_SIZING)
    }

    #[must_use]
    pub const fn position(mut self, position: Position) -> Self {
        self.0.position = position;
        self.mark(LayoutFields::POSITION)
    }

    #[must_use]
    pub fn with_layout(mut self, mutate: impl FnOnce(&mut StyleData)) -> Self {
        mutate(&mut self.0);
        self.mark(LayoutFields::ALL)
    }

    #[must_use]
    pub const fn data(&self) -> &StyleData {
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
