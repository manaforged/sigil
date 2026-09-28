#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::doc_markdown)]

use sigil::layout::{
    AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Position, Rect, Size, StyleData,
};

#[test]
fn public_flex_direction_variants_visible() {
    let _row = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        ..StyleData::default()
    };
    let _col = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        ..StyleData::default()
    };
    let _row_rev = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::RowReverse,
        ..StyleData::default()
    };
    let _col_rev = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::ColumnReverse,
        ..StyleData::default()
    };
}

#[test]
fn public_style_data_constructs_with_flex_subset() {
    let style = StyleData {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::Px(80.0),
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        align_self: Some(AlignSelf::End),
        size: Size::new(Dimension::Px(200.0), Dimension::Px(100.0)),
        padding: Rect::splat(LengthPercentage::Px(4.0)),
        margin: Rect::splat(LengthPercentageAuto::Px(2.0)),
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    };
    assert_eq!(style.display, Display::Flex);
    assert_eq!(style.flex_direction, FlexDirection::Row);
    assert!((style.flex_grow - 1.0).abs() < f32::EPSILON);
    assert_eq!(style.flex_basis, Dimension::Px(80.0));
}

#[test]
fn public_align_items_variants_distinct() {
    assert_ne!(AlignItems::Start, AlignItems::End);
    assert_ne!(AlignItems::Center, AlignItems::Stretch);
    assert_ne!(AlignItems::Baseline, AlignItems::Stretch);
}

#[test]
fn public_justify_content_variants_distinct() {
    assert_ne!(JustifyContent::Start, JustifyContent::End);
    assert_ne!(JustifyContent::SpaceBetween, JustifyContent::SpaceAround);
    assert_ne!(JustifyContent::SpaceEvenly, JustifyContent::Center);
}

#[test]
fn public_position_absolute_carries_through() {
    let style = StyleData {
        display: Display::Flex,
        position: Position::Absolute,
        ..StyleData::default()
    };
    assert_eq!(style.position, Position::Absolute);
}

#[test]
fn public_order_field_is_signed_integer() {
    let style = StyleData {
        display: Display::Flex,
        order: -3,
        ..StyleData::default()
    };
    assert_eq!(style.order, -3);
}
