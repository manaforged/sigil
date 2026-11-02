use super::*;
use crate::layout::style_data::{Axes, Dimension, Display, Edges, FlexDirection, Overflow};
use crate::layout::{cols, em, rows};

#[test]
fn text_lengths_are_dimension() {
    assert_eq!(rows(2.0), Dimension::Rows(2.0));
    assert_eq!(cols(4.0), Dimension::Cols(4.0));
    assert_eq!(em(1.5), Dimension::Em(1.5));
    let style = LayoutStyle::block().padding(Dimension::Percent(1.0));
    assert_eq!(style.data().padding.top, Dimension::Percent(1.0));
    let auto = LayoutStyle::block().padding(Dimension::Auto);
    assert_eq!(auto.data().padding.top, Dimension::Auto);
}

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
        Axes::new(Dimension::Px(320.0), Dimension::Px(200.0))
    );
    assert_eq!(d.gap, Axes::splat(Dimension::Px(4.0)));
}

#[test]
fn overflow_sets_both_axes() {
    let style = LayoutStyle::block().overflow(Overflow::Hidden);
    assert_eq!(style.data().overflow, Axes::splat(Overflow::Hidden));
}

#[test]
fn inset_sides_write_the_box() {
    let style = LayoutStyle::block()
        .inset_top(4.0)
        .inset_right(8.0)
        .inset_bottom(12.0)
        .inset_left(16.0);
    assert_eq!(
        style.data().inset,
        Edges {
            top: Dimension::Px(4.0),
            right: Dimension::Px(8.0),
            bottom: Dimension::Px(12.0),
            left: Dimension::Px(16.0)
        }
    );
}

#[test]
fn grow_pins_auto_min_size_to_zero() {
    let grown = LayoutStyle::flex_column().grow(1.0);
    assert_eq!(grown.data().flex_grow.to_bits(), 1.0f32.to_bits());
    assert_eq!(grown.data().min_size, Axes::splat(Dimension::Px(0.0)));
    let kept = LayoutStyle::flex_row().min_size(80.0, 24.0).grow(1.0);
    assert_eq!(
        kept.data().min_size,
        Axes::new(Dimension::Px(80.0), Dimension::Px(24.0))
    );
}
