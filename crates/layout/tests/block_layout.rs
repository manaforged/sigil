#![allow(unused_crate_dependencies)]
//! Integration smoke test for the P0.2 block layout body.
//!
//! The substantive block-layout tests (18 cases covering explicit
//! sizing, auto width, padding/border/margin, margin collapse,
//! absolute pinning, content-box vs border-box, min/max, percent,
//! `Display::None`, nested stacks) live at
//! `crates/layout/src/block_tests.rs` because `BlockSolver` and the
//! forest types are `pub(crate)` per DR-041 — integration tests
//! outside the crate cannot reach them without elevating a frozen API
//! surface. This file guards against accidental visibility regressions
//! in the public re-exports while the owned types land internally.

#![allow(missing_docs)]

// Workspace shared dev-deps surfaced into the integration target.
use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

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
    // Construct a realistic block-layout style atom through the
    // public re-exports. If any of these names regress to private,
    // the test fails to compile — that's the guard.
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
    // align-items and justify-content survive on the public surface
    // (Block doesn't read them, but the same StyleData is shared with
    // Flex; this guards their visibility for P0.3 consumers).
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
