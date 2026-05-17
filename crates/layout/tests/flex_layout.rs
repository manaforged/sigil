//! Integration smoke test for the P0.3 flex layout body.
//!
//! The substantive flex tests (43 cases covering direction, basis,
//! grow/shrink, wrap, justify-content, align-items, align-self,
//! align-content, gap, padding, margin, min/max, order, Display::None,
//! Position::Absolute) live at `crates/layout/src/flex_tests.rs`
//! because `FlexSolver` + forest types are `pub(crate)` per DR-041 —
//! integration tests outside the crate cannot reach them without
//! elevating a frozen API surface. This file guards against accidental
//! visibility regressions in the public re-exports while the owned
//! types land internally.

#![allow(missing_docs, clippy::doc_markdown)]

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
use taffy as _;

use sigil_layout::{
    BoxSizing, Dimension, Rect, Size, StyleAlignItems, StyleAlignSelf, StyleData, StyleDisplay,
    StyleFlexDirection, StyleJustifyContent, StyleLengthPercentage, StyleLengthPercentageAuto,
    StylePosition,
};

#[test]
fn public_flex_direction_variants_visible() {
    // Construct each direction through the public re-exports — if any
    // regresses to private, this file fails to compile.
    let _row = StyleData {
        display: StyleDisplay::Flex,
        flex_direction: StyleFlexDirection::Row,
        ..StyleData::default()
    };
    let _col = StyleData {
        display: StyleDisplay::Flex,
        flex_direction: StyleFlexDirection::Column,
        ..StyleData::default()
    };
    let _row_rev = StyleData {
        display: StyleDisplay::Flex,
        flex_direction: StyleFlexDirection::RowReverse,
        ..StyleData::default()
    };
    let _col_rev = StyleData {
        display: StyleDisplay::Flex,
        flex_direction: StyleFlexDirection::ColumnReverse,
        ..StyleData::default()
    };
}

#[test]
fn public_style_data_constructs_with_flex_subset() {
    let style = StyleData {
        display: StyleDisplay::Flex,
        flex_direction: StyleFlexDirection::Row,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::Px(80.0),
        justify_content: Some(StyleJustifyContent::SpaceBetween),
        align_items: Some(StyleAlignItems::Center),
        align_self: Some(StyleAlignSelf::End),
        size: Size::new(Dimension::Px(200.0), Dimension::Px(100.0)),
        padding: Rect::splat(StyleLengthPercentage::Px(4.0)),
        margin: Rect::splat(StyleLengthPercentageAuto::Px(2.0)),
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    };
    assert_eq!(style.display, StyleDisplay::Flex);
    assert_eq!(style.flex_direction, StyleFlexDirection::Row);
    assert!((style.flex_grow - 1.0).abs() < f32::EPSILON);
    assert_eq!(style.flex_basis, Dimension::Px(80.0));
}

#[test]
fn public_align_items_variants_distinct() {
    assert_ne!(StyleAlignItems::Start, StyleAlignItems::End);
    assert_ne!(StyleAlignItems::Center, StyleAlignItems::Stretch);
    assert_ne!(StyleAlignItems::Baseline, StyleAlignItems::Stretch);
}

#[test]
fn public_justify_content_variants_distinct() {
    assert_ne!(StyleJustifyContent::Start, StyleJustifyContent::End);
    assert_ne!(
        StyleJustifyContent::SpaceBetween,
        StyleJustifyContent::SpaceAround
    );
    assert_ne!(
        StyleJustifyContent::SpaceEvenly,
        StyleJustifyContent::Center
    );
}

#[test]
fn public_position_absolute_carries_through() {
    let style = StyleData {
        display: StyleDisplay::Flex,
        position: StylePosition::Absolute,
        ..StyleData::default()
    };
    assert_eq!(style.position, StylePosition::Absolute);
}

#[test]
fn public_order_field_is_signed_integer() {
    let style = StyleData {
        display: StyleDisplay::Flex,
        order: -3,
        ..StyleData::default()
    };
    assert_eq!(style.order, -3);
}
