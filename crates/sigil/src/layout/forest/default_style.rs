use crate::layout::StyleData;

pub(super) static DEFAULT_STYLE: StyleData = StyleData {
    display: crate::layout::style_data::Display::Block,
    position: crate::layout::style_data::Position::Relative,
    overflow: crate::layout::style_data::Size {
        width: crate::layout::style_data::Overflow::Visible,
        height: crate::layout::style_data::Overflow::Visible,
    },
    inset: crate::layout::style_data::Rect {
        left: crate::layout::style_data::LengthPercentageAuto::Auto,
        right: crate::layout::style_data::LengthPercentageAuto::Auto,
        top: crate::layout::style_data::LengthPercentageAuto::Auto,
        bottom: crate::layout::style_data::LengthPercentageAuto::Auto,
    },
    flex_direction: crate::layout::style_data::FlexDirection::Row,
    flex_wrap: crate::layout::style_data::FlexWrap::NoWrap,
    flex_grow: 0.0,
    flex_shrink: 1.0,
    flex_basis: crate::layout::style_data::Dimension::Auto,
    justify_content: None,
    align_items: None,
    align_content: None,
    align_self: None,
    gap: crate::layout::style_data::Size {
        width: crate::layout::style_data::LengthPercentage::Px(0.0),
        height: crate::layout::style_data::LengthPercentage::Px(0.0),
    },
    margin: crate::layout::style_data::Rect {
        left: crate::layout::style_data::LengthPercentageAuto::Px(0.0),
        right: crate::layout::style_data::LengthPercentageAuto::Px(0.0),
        top: crate::layout::style_data::LengthPercentageAuto::Px(0.0),
        bottom: crate::layout::style_data::LengthPercentageAuto::Px(0.0),
    },
    padding: crate::layout::style_data::Rect {
        left: crate::layout::style_data::LengthPercentage::Px(0.0),
        right: crate::layout::style_data::LengthPercentage::Px(0.0),
        top: crate::layout::style_data::LengthPercentage::Px(0.0),
        bottom: crate::layout::style_data::LengthPercentage::Px(0.0),
    },
    border: crate::layout::style_data::Rect {
        left: crate::layout::style_data::LengthPercentage::Px(0.0),
        right: crate::layout::style_data::LengthPercentage::Px(0.0),
        top: crate::layout::style_data::LengthPercentage::Px(0.0),
        bottom: crate::layout::style_data::LengthPercentage::Px(0.0),
    },
    size: crate::layout::style_data::Size {
        width: crate::layout::style_data::Dimension::Auto,
        height: crate::layout::style_data::Dimension::Auto,
    },
    min_size: crate::layout::style_data::Size {
        width: crate::layout::style_data::Dimension::Auto,
        height: crate::layout::style_data::Dimension::Auto,
    },
    max_size: crate::layout::style_data::Size {
        width: crate::layout::style_data::Dimension::Auto,
        height: crate::layout::style_data::Dimension::Auto,
    },
    order: 0,
    box_sizing: crate::layout::style_data::BoxSizing::ContentBox,
    grid_template_columns: Vec::new(),
    grid_template_rows: Vec::new(),
    grid_column_span: 1,
    grid_row_span: 1,
    grid_column_start: 0,
    grid_row_start: 0,
    grid_auto_flow_dense: false,
    direction: crate::style::Direction::Ltr,
};
