use super::*;
use crate::style_data::{Display, FlexDirection, LengthPercentage, LengthPercentageAuto, Overflow};

#[test]
fn pane_sugar_matches_manual_chain() {
    let sugared = LayoutStyle::pane(320.0, 200.0);
    let manual = LayoutStyle::flex_column().fixed_size(320.0, 200.0).gap(4.0);

    assert_eq!(sugared.data(), manual.data());
    assert_eq!(sugared.data().display, Display::Flex);
    assert_eq!(sugared.data().flex_direction, FlexDirection::Column);
    assert_eq!(sugared.data().size.width, Dimension::Px(320.0));
    assert_eq!(sugared.data().size.height, Dimension::Px(200.0));
    assert_eq!(sugared.data().gap.width, LengthPercentage::Px(4.0));
    assert_eq!(sugared.data().gap.height, LengthPercentage::Px(4.0));
}

#[test]
fn overflow_sets_both_axes() {
    let style = LayoutStyle::block().overflow(Overflow::Hidden);
    assert_eq!(style.data().overflow.width, Overflow::Hidden);
    assert_eq!(style.data().overflow.height, Overflow::Hidden);
}

#[test]
fn inset_sides_write_the_box() {
    let style = LayoutStyle::block()
        .inset_top(4.0)
        .inset_right(8.0)
        .inset_bottom(12.0)
        .inset_left(16.0);
    assert_eq!(style.data().inset.top, LengthPercentageAuto::Px(4.0));
    assert_eq!(style.data().inset.right, LengthPercentageAuto::Px(8.0));
    assert_eq!(style.data().inset.bottom, LengthPercentageAuto::Px(12.0));
    assert_eq!(style.data().inset.left, LengthPercentageAuto::Px(16.0));
}

#[test]
fn grow_pins_auto_min_size_to_zero() {
    let grown = LayoutStyle::flex_column().grow(1.0);
    assert_eq!(grown.data().flex_grow.to_bits(), 1.0f32.to_bits());
    assert_eq!(grown.data().min_size.width, Dimension::Px(0.0));
    assert_eq!(grown.data().min_size.height, Dimension::Px(0.0));
    let kept = LayoutStyle::flex_row().min_size(80.0, 24.0).grow(1.0);
    assert_eq!(kept.data().min_size.width, Dimension::Px(80.0));
    assert_eq!(kept.data().min_size.height, Dimension::Px(24.0));
}
