#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! W1.4 — verify the round-trip from `StyleAtom::OverflowX` / `OverflowY`
//! through the style adapter into `StyleData::overflow`.
//!
//! Renderer-side clipping behavior is out of scope here — this test
//! only proves the storage path: atoms in → resolved `StyleData` carries
//! the matching `Overflow` variant on the right axis.

#![allow(
    clippy::unwrap_used,
    clippy::missing_docs_in_private_items,
    missing_docs
)]

// Workspace dev-deps that other layout tests reach for; keep the unused
// list quiet under -D warnings.

use sigil_layout::{LayoutStyle, Overflow, style_adapter::apply_style_atoms};
use sigil_style::{OverflowKind, StyleAtom};

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
