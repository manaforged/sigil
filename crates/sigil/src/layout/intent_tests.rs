use super::*;
use crate::layout::style_data::{
    Dimension, Display, FlexDirection, LengthPercentage, LengthPercentageAuto, Overflow, Rect, Size,
};

#[test]
fn pane_sugar_matches_manual_chain() {
    let sugared = LayoutStyle::pane(320.0, 200.0);
    let manual = LayoutStyle::flex_column().fixed_size(320.0, 200.0).gap(4.0);
    let d = sugared.data();
    assert_eq!(d, manual.data());
    assert_eq!(
        (d.display, d.flex_direction),
        (Display::Flex, FlexDirection::Column)
    );
    assert_eq!(
        d.size,
        Size::new(Dimension::Px(320.0), Dimension::Px(200.0))
    );
    assert_eq!(d.gap, Size::splat(LengthPercentage::Px(4.0)));
}

#[test]
fn overflow_sets_both_axes() {
    let style = LayoutStyle::block().overflow(Overflow::Hidden);
    assert_eq!(style.data().overflow, Size::splat(Overflow::Hidden));
}

#[test]
fn inset_sides_write_the_box() {
    let style = LayoutStyle::block()
        .inset_top(4.0)
        .inset_right(8.0)
        .inset_bottom(12.0)
        .inset_left(16.0);
    let px = LengthPercentageAuto::Px;
    assert_eq!(
        style.data().inset,
        Rect {
            top: px(4.0),
            right: px(8.0),
            bottom: px(12.0),
            left: px(16.0)
        }
    );
}

#[test]
fn grow_pins_auto_min_size_to_zero() {
    let grown = LayoutStyle::flex_column().grow(1.0);
    assert_eq!(grown.data().flex_grow.to_bits(), 1.0f32.to_bits());
    assert_eq!(grown.data().min_size, Size::splat(Dimension::Px(0.0)));
    let kept = LayoutStyle::flex_row().min_size(80.0, 24.0).grow(1.0);
    assert_eq!(
        kept.data().min_size,
        Size::new(Dimension::Px(80.0), Dimension::Px(24.0))
    );
}
