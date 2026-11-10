#![allow(clippy::unwrap_used)]

use sigil::layout::{LayoutStyle, Overflow, apply_style_atoms};
use sigil::style::StyleAtom;

#[test]
fn overflow_x_hidden_round_trips_through_style_adapter() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(&[StyleAtom::OverflowX(Overflow::Hidden)], &mut style);
    assert_eq!(
        sigil::testing::style_data(&style).overflow.width,
        Overflow::Hidden
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.height,
        Overflow::Visible
    );
}

#[test]
fn overflow_y_scroll_round_trips_through_style_adapter() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(&[StyleAtom::OverflowY(Overflow::Scroll)], &mut style);
    assert_eq!(
        sigil::testing::style_data(&style).overflow.height,
        Overflow::Scroll
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.width,
        Overflow::Visible
    );
}

#[test]
fn both_axes_resolve_independently() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(
        &[
            StyleAtom::OverflowX(Overflow::Hidden),
            StyleAtom::OverflowY(Overflow::Scroll),
        ],
        &mut style,
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.width,
        Overflow::Hidden
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.height,
        Overflow::Scroll
    );
}

#[test]
fn later_overflow_atom_wins_on_same_axis() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(
        &[
            StyleAtom::OverflowX(Overflow::Scroll),
            StyleAtom::OverflowX(Overflow::Hidden),
        ],
        &mut style,
    );
    assert_eq!(
        sigil::testing::style_data(&style).overflow.width,
        Overflow::Hidden
    );
}
