use super::*;
use crate::layout::Space;
use crate::layout::style_data::{Axes, Display, Edges, FlexDirection, Length, Overflow};
use crate::layout::{cols, em, rows};

#[test]
fn text_lengths_are_dimension() {
    assert_eq!(rows(2.0), Length::Rows(2.0));
    assert_eq!(cols(4.0), Length::Cols(4.0));
    assert_eq!(em(1.5), Length::Em(1.5));
    let style = LayoutStyle::block().padding(Length::Percent(1.0));
    assert_eq!(style.data().padding.top, Length::Percent(1.0));
    let auto = LayoutStyle::block().padding(Length::Auto);
    assert_eq!(auto.data().padding.top, Length::Auto);
}

#[test]
fn pane_sugar_matches_manual_chain() {
    let sugared = LayoutStyle::pane(320.0, 200.0);
    let manual = LayoutStyle::flex_column()
        .fixed_size(320.0, 200.0)
        .gap(Space::Xs);
    let d = sugared.data();
    assert_eq!(d, manual.data());
    assert_eq!(
        (d.display, d.flex_direction),
        (Display::Flex, FlexDirection::Column)
    );
    assert_eq!(d.size, Axes::new(Length::Px(320.0), Length::Px(200.0)));
    assert_eq!(d.gap, Axes::splat(Length::Px(4.0)));
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
            top: Length::Px(4.0),
            right: Length::Px(8.0),
            bottom: Length::Px(12.0),
            left: Length::Px(16.0)
        }
    );
}

#[test]
fn grow_pins_auto_min_size_to_zero() {
    let grown = LayoutStyle::flex_column().grow(1.0);
    assert_eq!(grown.data().flex_grow.to_bits(), 1.0f32.to_bits());
    assert_eq!(grown.data().min_size, Axes::splat(Length::Px(0.0)));
    let kept = LayoutStyle::flex_row().min_size(80.0, 24.0).grow(1.0);
    assert_eq!(
        kept.data().min_size,
        Axes::new(Length::Px(80.0), Length::Px(24.0))
    );
}
