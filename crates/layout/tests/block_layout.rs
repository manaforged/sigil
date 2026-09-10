#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil_layout::{
    BoxSizing, Dimension, Rect, Size, StyleAlignItems, StyleData, StyleDisplay, StyleFlexDirection,
    StyleJustifyContent, StyleLengthPercentage, StyleLengthPercentageAuto, StylePosition,
};

#[test]
fn public_style_data_default_matches_css_baseline() {
    let s = StyleData::default();
    assert_eq!(s.display, StyleDisplay::Block);
    assert_eq!(s.position, StylePosition::Relative);
    assert_eq!(s.box_sizing, BoxSizing::ContentBox);
}

#[test]
fn public_style_data_is_constructible_with_block_subset() {
    let style = StyleData {
        display: StyleDisplay::Block,
        position: StylePosition::Relative,
        flex_direction: StyleFlexDirection::Row,
        size: Size::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        padding: Rect::splat(StyleLengthPercentage::Px(4.0)),
        margin: Rect::splat(StyleLengthPercentageAuto::Px(2.0)),
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    };
    assert_eq!(style.size.width, Dimension::Px(100.0));
    assert_eq!(style.box_sizing, BoxSizing::BorderBox);
}

#[test]
fn public_style_data_carries_min_max_clamps() {
    let style = StyleData {
        min_size: Size::new(Dimension::Px(20.0), Dimension::Auto),
        max_size: Size::new(Dimension::Px(200.0), Dimension::Px(400.0)),
        ..StyleData::default()
    };
    assert_eq!(style.min_size.width, Dimension::Px(20.0));
    assert_eq!(style.max_size.height, Dimension::Px(400.0));
}

#[test]
fn public_re_exports_cover_align_and_justify() {
    let style = StyleData {
        align_items: Some(StyleAlignItems::Center),
        justify_content: Some(StyleJustifyContent::SpaceBetween),
        ..StyleData::default()
    };
    assert_eq!(style.align_items, Some(StyleAlignItems::Center));
    assert_eq!(
        style.justify_content,
        Some(StyleJustifyContent::SpaceBetween)
    );
}
