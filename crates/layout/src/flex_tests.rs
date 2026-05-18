//! Flex-layout integration tests. Lives in-crate (not under `tests/`)
//! because `FlexSolver` + forest types are `pub(crate)` per DR-041 —
//! integration tests outside the crate can't reach them without
//! elevating a frozen API surface.
//!
//! Coverage matches the P0.3 (W2.4) brief:
//! * direction (Row, Column, RowReverse, ColumnReverse)
//! * flex-basis with explicit px / percent / auto
//! * flex-grow / flex-shrink distribution
//! * flex-wrap (NoWrap, Wrap, WrapReverse)
//! * justify-content (Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly)
//! * align-items / align-self (Start, End, Center, Stretch)
//! * align-content for multi-line
//! * gap (main + cross)
//! * padding + margin offset placement
//! * min/max main clamping
//! * order rearrangement
//! * Display::None skipped, Position::Absolute isolated

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

use super::*;
use crate::flex_algorithm::{Edges as AlgoEdges, clamp_minmax, dim_to_border_box, resolve_lp};
use crate::forest::{AvailableSpace, LayoutForest, MeasureScope};
use crate::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Position, Rect, Size as StyleSize,
    StyleData,
};
use sigil_hal::{LogicalSize, Size as HalSize};
use sigil_kernel::TextContent;

const TOL: f32 = 0.01;

struct NullText;
impl crate::TextMeasure for NullText {
    fn measure(&self, _: &TextContent, _: HalSize) -> LogicalSize {
        LogicalSize::default()
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

fn run(forest: &LayoutForest, root: crate::forest::LocalId, w: f32, h: f32) -> FlexSolver<'_> {
    let measurer: &'static NullText = &NullText;
    let scope = MeasureScope::new(forest, measurer, AvailableSpace::definite(w, h));
    let solver = FlexSolver::new(scope);
    solver.solve(forest.refer(root).unwrap()).unwrap();
    solver
}

fn flex_parent() -> StyleData {
    StyleData {
        display: Display::Flex,
        ..StyleData::default()
    }
}

fn fixed(w: f32, h: f32) -> StyleData {
    StyleData {
        size: StyleSize::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
}

fn add_child(
    f: &mut LayoutForest,
    parent: crate::forest::LocalId,
    style: StyleData,
) -> crate::forest::LocalId {
    let id = f.insert(None, style);
    f.attach(parent, id).unwrap();
    id
}

// ---------------------------------------------------------------------
// Row direction — placement + basic grow
// ---------------------------------------------------------------------

#[test]
fn row_lays_children_along_inline_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(60.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.x, 0.0), "ba.x={}", ba.x);
    assert!(near(ba.y, 0.0));
    assert!(near(bb.x, 40.0), "bb.x={}", bb.x);
    assert!(near(bb.y, 0.0));
}

#[test]
fn column_lays_children_along_block_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(300.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(40.0, 50.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.y, 0.0));
    assert!(near(bb.y, 30.0), "bb.y={}", bb.y);
}

#[test]
fn row_reverse_reverses_main_order() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::RowReverse,
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let b = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // Reverse: b laid first at x=0, then a after b.
    assert!(near(bb.x, 0.0), "bb.x={}", bb.x);
    assert!(near(ba.x, 60.0), "ba.x={}", ba.x);
}

#[test]
fn column_reverse_reverses_block_order() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_direction: FlexDirection::ColumnReverse,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let b = add_child(&mut f, parent, fixed(40.0, 50.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(bb.y, 0.0));
    assert!(near(ba.y, 50.0), "ba.y={}", ba.y);
}

// ---------------------------------------------------------------------
// flex-basis / grow / shrink
// ---------------------------------------------------------------------

#[test]
fn flex_basis_overrides_size_for_main_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(10.0), Dimension::Px(20.0)),
        flex_basis: Dimension::Px(80.0),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.w, 80.0), "ba.w={}", ba.w);
}

#[test]
fn flex_grow_distributes_free_space_proportionally() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 1.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let b_style = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 2.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let b = add_child(&mut f, parent, b_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // a: 1/3 of 300 = 100, b: 2/3 = 200.
    assert!(near(ba.w, 100.0), "ba.w={}", ba.w);
    assert!(near(bb.w, 200.0), "bb.w={}", bb.w);
}

#[test]
fn flex_shrink_distributes_deficit_proportionally() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    // Two items basis 80 each, total 160, overflow 60.
    // Equal shrink -> remove 30 each -> 50 each.
    let a_style = StyleData {
        flex_basis: Dimension::Px(80.0),
        flex_shrink: 1.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.w, 50.0), "ba.w={}", ba.w);
    assert!(near(bb.w, 50.0), "bb.w={}", bb.w);
}

#[test]
fn no_grow_no_shrink_keeps_basis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(400.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(60.0),
        flex_grow: 0.0,
        flex_shrink: 0.0,
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.w, 60.0));
}

// ---------------------------------------------------------------------
// justify-content (main axis)
// ---------------------------------------------------------------------

#[test]
fn justify_start_keeps_items_packed_at_main_start() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Start),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 0.0));
}

#[test]
fn justify_end_packs_items_at_main_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::End),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.x, 240.0), "ba.x={}", ba.x);
}

#[test]
fn justify_center_centers_items_on_main_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(100.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 100.0));
}

#[test]
fn justify_space_between_distributes_gaps_between_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceBetween),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(ba.x, 0.0));
    assert!(near(bb.x, 250.0), "bb.x={}", bb.x);
}

#[test]
fn justify_space_around_pads_edges_half() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceAround),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // Free=200, /2 items = 100, /2 outer = 50.
    assert!(near(ba.x, 50.0), "ba.x={}", ba.x);
    assert!(near(bb.x, 200.0), "bb.x={}", bb.x);
}

#[test]
fn justify_space_evenly_pads_all_gaps_equally() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            justify_content: Some(JustifyContent::SpaceEvenly),
            size: StyleSize::new(Dimension::Px(310.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(50.0, 20.0));
    let b = add_child(&mut f, parent, fixed(50.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    // free=210, slots=3 -> 70 each.
    assert!(near(s.boxed(a).unwrap().x, 70.0));
    assert!(near(s.boxed(b).unwrap().x, 190.0));
}

// ---------------------------------------------------------------------
// align-items / align-self (cross axis)
// ---------------------------------------------------------------------

#[test]
fn align_items_start_keeps_items_at_cross_start() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Start),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 0.0));
}

#[test]
fn align_items_end_pins_items_to_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::End),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 70.0));
}

#[test]
fn align_items_center_centers_items_cross_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Center),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 30.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 35.0));
}

#[test]
fn align_items_stretch_fills_cross_when_cross_size_auto() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Stretch),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Auto),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.h, 100.0), "ba.h={}", ba.h);
}

#[test]
fn align_self_overrides_container_align_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            align_items: Some(AlignItems::Start),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(30.0)),
        align_self: Some(AlignSelf::End),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 70.0));
}

// ---------------------------------------------------------------------
// wrap + multi-line cross distribution
// ---------------------------------------------------------------------

#[test]
fn nowrap_keeps_overflowing_items_on_single_line() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::NoWrap,
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    // Three 80-px items in a 150-px line.
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(20.0)),
        flex_shrink: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style.clone());
    let c = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    // All three on the same cross-axis row (y=0).
    assert!(near(s.boxed(a).unwrap().y, 0.0));
    assert!(near(s.boxed(b).unwrap().y, 0.0));
    assert!(near(s.boxed(c).unwrap().y, 0.0));
}

#[test]
fn wrap_breaks_overflowing_items_onto_next_line() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style.clone());
    let c = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    // a + b on line 0 would overflow — actually a alone takes 80 of 150, b
    // would push to 160 > 150 so b wraps to line 1. c also wraps.
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    let bc = s.boxed(c).unwrap();
    assert!(near(ba.y, 0.0));
    assert!(near(bb.y, 30.0), "bb.y={}", bb.y);
    assert!(near(bc.y, 60.0), "bc.y={}", bc.y);
}

#[test]
fn wrap_reverse_orders_lines_from_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::WrapReverse,
            align_content: Some(AlignContent::Start),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    // Lines reversed: b's line first.
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    assert!(near(bb.y, 0.0), "bb.y={}", bb.y);
    assert!(near(ba.y, 30.0), "ba.y={}", ba.y);
}

#[test]
fn align_content_center_centers_lines_cross_axis() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            align_content: Some(AlignContent::Center),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    // Two lines, each 30 tall = 60. Container 200 -> free 140 -> center origin 70.
    assert!(near(s.boxed(a).unwrap().y, 70.0));
    assert!(near(s.boxed(b).unwrap().y, 100.0));
}

#[test]
fn align_content_end_packs_lines_at_cross_end() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            align_content: Some(AlignContent::End),
            size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(200.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(30.0)),
        flex_shrink: 0.0,
        flex_grow: 0.0,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().y, 140.0));
    assert!(near(s.boxed(b).unwrap().y, 170.0));
}

// ---------------------------------------------------------------------
// gap, padding, margin
// ---------------------------------------------------------------------

#[test]
fn gap_inserts_main_axis_gutter_between_items() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            gap: StyleSize::new(LengthPercentage::Px(8.0), LengthPercentage::Px(0.0)),
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let b = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 0.0));
    assert!(
        near(s.boxed(b).unwrap().x, 48.0),
        "b.x={}",
        s.boxed(b).unwrap().x
    );
}

#[test]
fn padding_offsets_child_origin() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            padding: Rect::splat(LengthPercentage::Px(10.0)),
            size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(near(ba.x, 10.0));
    assert!(near(ba.y, 10.0));
}

#[test]
fn margin_on_item_offsets_main_position() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(15.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(0.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().x, 15.0));
}

// ---------------------------------------------------------------------
// min/max clamping
// ---------------------------------------------------------------------

#[test]
fn min_size_floors_main_axis_shrink() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(80.0),
        flex_shrink: 1.0,
        min_size: StyleSize::new(Dimension::Px(70.0), Dimension::Auto),
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style.clone());
    let b = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // Floor 70 each; without floor would be 50.
    assert!(ba.w >= 70.0 - TOL);
    assert!(bb.w >= 70.0 - TOL);
}

#[test]
fn max_size_caps_main_axis_grow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        flex_basis: Dimension::Px(0.0),
        flex_grow: 1.0,
        max_size: StyleSize::new(Dimension::Px(80.0), Dimension::Auto),
        size: StyleSize::new(Dimension::Auto, Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    let ba = s.boxed(a).unwrap();
    assert!(ba.w <= 80.0 + TOL, "ba.w={}", ba.w);
}

// ---------------------------------------------------------------------
// order, Display::None, Position::Absolute
// ---------------------------------------------------------------------

#[test]
fn order_reorders_items_by_ascending_order_field() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        order: 10,
        ..StyleData::default()
    };
    let b_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        order: -5,
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let b = add_child(&mut f, parent, b_style);
    let s = run(&f, parent, 400.0, 400.0);
    // b has lower order, lays first.
    assert!(near(s.boxed(b).unwrap().x, 0.0));
    assert!(near(s.boxed(a).unwrap().x, 40.0));
}

#[test]
fn display_none_items_are_skipped() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let hidden_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        display: Display::None,
        ..StyleData::default()
    };
    let _hidden = add_child(&mut f, parent, hidden_style);
    let visible = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    // Visible lays at 0 (hidden does not contribute).
    assert!(near(s.boxed(visible).unwrap().x, 0.0));
}

#[test]
fn absolute_items_are_isolated_from_flex_flow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(300.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let abs_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        position: Position::Absolute,
        ..StyleData::default()
    };
    let _abs = add_child(&mut f, parent, abs_style);
    let flow = add_child(&mut f, parent, fixed(60.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    // Flow item lays at 0 (absolute does not advance main cursor).
    assert!(near(s.boxed(flow).unwrap().x, 0.0));
}

// ---------------------------------------------------------------------
// container output box
// ---------------------------------------------------------------------

#[test]
fn container_box_picks_up_explicit_size() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(220.0), Dimension::Px(70.0)),
            ..StyleData::default()
        },
    );
    let _a = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let b = s.boxed(parent).unwrap();
    assert!(near(b.w, 220.0));
    assert!(near(b.h, 70.0));
}

#[test]
fn take_drains_results_and_resets_internal_map() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, flex_parent());
    let _a = add_child(&mut f, parent, fixed(20.0, 20.0));
    let s = run(&f, parent, 200.0, 100.0);
    let drained = s.take();
    assert!(!drained.is_empty());
    // After draining, the solver's internal map is empty.
    let drained2 = s.take();
    assert!(drained2.is_empty());
}

#[test]
fn empty_flex_container_produces_only_self_box() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
            ..StyleData::default()
        },
    );
    let s = run(&f, parent, 200.0, 100.0);
    let drained = s.take();
    assert_eq!(drained.len(), 1);
    assert!(drained.contains_key(&parent));
}

// ---------------------------------------------------------------------
// helper unit tests
// ---------------------------------------------------------------------

#[test]
fn clamp_minmax_min_floors_above_max() {
    assert!(near(clamp_minmax(80.0, Some(120.0), Some(100.0)), 120.0));
}

#[test]
fn clamp_minmax_no_bounds_passes_through() {
    assert!(near(clamp_minmax(42.0, None, None), 42.0));
}

#[test]
fn resolve_lp_percent_resolves_against_basis() {
    let v = resolve_lp(LengthPercentage::Percent(0.25), 200.0);
    assert!(near(v, 50.0));
}

#[test]
fn dim_to_border_box_adds_chrome_in_content_box_mode() {
    let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::ContentBox);
    assert_eq!(v, Some(120.0));
}

#[test]
fn dim_to_border_box_skips_chrome_in_border_box_mode() {
    let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::BorderBox);
    assert_eq!(v, Some(100.0));
}

#[test]
fn dim_to_border_box_returns_none_for_auto() {
    assert!(dim_to_border_box(Dimension::Auto, 100.0, 0.0, BoxSizing::ContentBox).is_none());
}

#[test]
fn algo_edges_inline_and_block_sum_match_struct_fields() {
    let e = AlgoEdges {
        left: 1.0,
        right: 2.0,
        top: 3.0,
        bottom: 4.0,
    };
    assert!(near(e.inline(), 3.0));
    assert!(near(e.block(), 7.0));
}

#[test]
fn percent_dimension_resolves_against_container() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            display: Display::Flex,
            size: StyleSize::new(Dimension::Px(400.0), Dimension::Px(100.0)),
            ..StyleData::default()
        },
    );
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Percent(0.25), Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let a = add_child(&mut f, parent, a_style);
    let s = run(&f, parent, 400.0, 400.0);
    assert!(near(s.boxed(a).unwrap().w, 100.0));
}
