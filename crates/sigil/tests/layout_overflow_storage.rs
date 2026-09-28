#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::layout::{LayoutStyle, Overflow, apply_style_atoms};
use sigil::style::{OverflowKind, StyleAtom};

#[test]
fn overflow_x_hidden_round_trips_through_style_adapter() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(&[StyleAtom::OverflowX(OverflowKind::Hidden)], &mut style);
    assert_eq!(style.data().overflow.width, Overflow::Hidden);
    assert_eq!(style.data().overflow.height, Overflow::Visible);
}

#[test]
fn overflow_y_scroll_round_trips_through_style_adapter() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(&[StyleAtom::OverflowY(OverflowKind::Scroll)], &mut style);
    assert_eq!(style.data().overflow.height, Overflow::Scroll);
    assert_eq!(style.data().overflow.width, Overflow::Visible);
}

#[test]
fn both_axes_resolve_independently() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(
        &[
            StyleAtom::OverflowX(OverflowKind::Hidden),
            StyleAtom::OverflowY(OverflowKind::Scroll),
        ],
        &mut style,
    );
    assert_eq!(style.data().overflow.width, Overflow::Hidden);
    assert_eq!(style.data().overflow.height, Overflow::Scroll);
}

#[test]
fn later_overflow_atom_wins_on_same_axis() {
    let mut style = LayoutStyle::block();
    apply_style_atoms(
        &[
            StyleAtom::OverflowX(OverflowKind::Scroll),
            StyleAtom::OverflowX(OverflowKind::Hidden),
        ],
        &mut style,
    );
    assert_eq!(style.data().overflow.width, Overflow::Hidden);
}
