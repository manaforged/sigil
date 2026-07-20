//! Block-layout integration tests. Lives in-crate (not under `tests/`)
//! because `BlockSolver` + the forest types are `pub(crate)` per
//! DR-041 — integration tests outside the crate can't reach them
//! without elevating the visibility of a frozen API surface.
//!
//! Coverage matches the P0.2 (W2.3) brief:
//! * single block with explicit size
//! * `width: auto` fills containing block
//! * padding / border / margin offset placement
//! * adjacent-sibling margin collapse (positive, mixed sign)
//! * `Position::Absolute` isolated from in-flow stacking
//! * content-box vs border-box sizing
//! * min / max clamping
//! * `Display::None` skipped
//! * percent dimensions
//! * nested blocks

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use super::*;
use crate::forest::{AvailableSpace, LayoutForest, MeasureScope};
use crate::style_data::{
    BoxSizing, Dimension, Display, LengthPercentage, LengthPercentageAuto, Position, Rect,
    Size as StyleSize, StyleData,
};
use sigil_hal::{LogicalSize, Size as HalSize};
use sigil_kernel::TextContent;

const TOL: f32 = 0.001;

struct NullText;
impl crate::TextMeasure for NullText {
    fn measure(&self, _: &TextContent, _: HalSize) -> LogicalSize {
        LogicalSize::default()
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

fn run(forest: &LayoutForest, root: crate::forest::LocalId, w: f32, h: f32) -> BlockSolver<'_> {
    let measurer: &'static NullText = &NullText;
    let scope = MeasureScope::new(forest, measurer, AvailableSpace::definite(w, h));
    let solver = BlockSolver::new(scope);
    solver.solve(forest.refer(root).unwrap()).unwrap();
    solver
}

#[test]
fn explicit_width_and_height_are_honoured() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(150.0), Dimension::Px(60.0)),
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 150.0));
    assert!(near(b.h, 60.0));
    assert!(near(b.x, 0.0));
    assert!(near(b.y, 0.0));
}

#[test]
fn auto_width_fills_containing_block() {
    let mut f = LayoutForest::new();
    let root = f.insert(None, StyleData::default());
    let s = run(&f, root, 800.0, 600.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 800.0));
}

#[test]
fn padding_contributes_to_auto_height() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        padding: Rect {
            left: LengthPercentage::Px(8.0),
            right: LengthPercentage::Px(8.0),
            top: LengthPercentage::Px(12.0),
            bottom: LengthPercentage::Px(4.0),
        },
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 200.0, 150.0);
    let b = s.boxed(root).unwrap();
    // No children — auto height is top+bottom padding (and 0 content).
    assert!(near(b.h, 16.0));
}

#[test]
fn child_origin_offsets_by_padding() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(200.0)),
        ..StyleData::default()
    };
    let child_style = StyleData {
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let child = f.insert(None, child_style);
    f.attach(parent, child).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let cb = s.boxed(child).unwrap();
    assert!(near(cb.x, 10.0));
    assert!(near(cb.y, 10.0));
}

#[test]
fn adjacent_sibling_top_bottom_margins_collapse() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        size: StyleSize::new(Dimension::Px(200.0), Dimension::Auto),
        ..StyleData::default()
    };
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(30.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(0.0),
            bottom: LengthPercentageAuto::Px(20.0),
        },
        ..StyleData::default()
    };
    let b_style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(30.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(12.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let a = f.insert(None, a_style);
    let b = f.insert(None, b_style);
    f.attach(parent, a).unwrap();
    f.attach(parent, b).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // a sits flush with parent's content top (parent has zero
    // padding/border; outer top of `a` is 0 because the first child's
    // top-margin lands raw against the sealed parent in this subset).
    assert!(near(ba.y, 0.0));
    // Gap between a's bottom edge and b's top edge = max(20, 12) = 20.
    assert!(near(bb.y, ba.y + ba.h + 20.0));
}

#[test]
fn mixed_sign_margins_sum() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(40.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(0.0),
            bottom: LengthPercentageAuto::Px(15.0),
        },
        ..StyleData::default()
    };
    let b_style = StyleData {
        size: StyleSize::new(Dimension::Px(80.0), Dimension::Px(40.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(-5.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let a = f.insert(None, a_style);
    let b = f.insert(None, b_style);
    f.attach(parent, a).unwrap();
    f.attach(parent, b).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // 15 + (-5) = 10 gap.
    assert!(near(bb.y, ba.y + ba.h + 10.0));
}

#[test]
fn absolute_child_does_not_advance_flow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let abs_style = StyleData {
        position: Position::Absolute,
        size: StyleSize::new(Dimension::Px(40.0), Dimension::Px(40.0)),
        ..StyleData::default()
    };
    let flow_style = StyleData {
        size: StyleSize::new(Dimension::Px(60.0), Dimension::Px(20.0)),
        ..StyleData::default()
    };
    let abs = f.insert(None, abs_style);
    let flow = f.insert(None, flow_style);
    f.attach(parent, abs).unwrap();
    f.attach(parent, flow).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let bf = s.boxed(flow).unwrap();
    // Absolute first child must NOT push the in-flow sibling down.
    assert!(near(bf.y, 0.0));
    // Absolute child still placed (sized + recorded).
    let ba = s.boxed(abs).unwrap();
    assert!(near(ba.w, 40.0));
    assert!(near(ba.h, 40.0));
}

#[test]
fn border_box_width_includes_padding() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        box_sizing: BoxSizing::BorderBox,
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    // BorderBox: 100 is the OUTER (border-box) width.
    assert!(near(b.w, 100.0));
    assert!(near(b.h, 50.0));
}

#[test]
fn content_box_width_adds_padding() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(100.0), Dimension::Px(50.0)),
        padding: Rect::splat(LengthPercentage::Px(10.0)),
        box_sizing: BoxSizing::ContentBox,
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    // ContentBox: 100 is content width, border-box = 100 + 2*10.
    assert!(near(b.w, 120.0));
    assert!(near(b.h, 70.0));
}

#[test]
fn min_width_floors_below_preferred() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(50.0)),
        min_size: StyleSize::new(Dimension::Px(100.0), Dimension::Auto),
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.w, 100.0));
}

#[test]
fn max_height_caps_above_preferred() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(500.0)),
        max_size: StyleSize::new(Dimension::Auto, Dimension::Px(120.0)),
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.h, 120.0));
}

#[test]
fn percent_width_resolves_against_containing_block() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        size: StyleSize::new(Dimension::Px(400.0), Dimension::Px(200.0)),
        ..StyleData::default()
    };
    let child_style = StyleData {
        size: StyleSize::new(Dimension::Percent(0.5), Dimension::Px(30.0)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let child = f.insert(None, child_style);
    f.attach(parent, child).unwrap();
    let s = run(&f, parent, 1000.0, 800.0);
    let bc = s.boxed(child).unwrap();
    assert!(near(bc.w, 200.0));
}

#[test]
fn display_none_child_is_skipped() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let hidden_style = StyleData {
        display: Display::None,
        size: StyleSize::new(Dimension::Px(99.0), Dimension::Px(99.0)),
        ..StyleData::default()
    };
    let visible_style = StyleData {
        size: StyleSize::new(Dimension::Px(20.0), Dimension::Px(10.0)),
        ..StyleData::default()
    };
    let hidden = f.insert(None, hidden_style);
    let visible = f.insert(None, visible_style);
    f.attach(parent, hidden).unwrap();
    f.attach(parent, visible).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    // Hidden node never visited.
    assert!(s.boxed(hidden).is_none());
    // Visible sits flush with the parent's content origin.
    let bv = s.boxed(visible).unwrap();
    assert!(near(bv.y, 0.0));
}

#[test]
fn nested_blocks_stack_vertically() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        size: StyleSize::new(Dimension::Px(200.0), Dimension::Auto),
        ..StyleData::default()
    };
    let child_style = |h: f32| StyleData {
        size: StyleSize::new(Dimension::Auto, Dimension::Px(h)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let a = f.insert(None, child_style(40.0));
    let b = f.insert(None, child_style(25.0));
    let c = f.insert(None, child_style(15.0));
    f.attach(parent, a).unwrap();
    f.attach(parent, b).unwrap();
    f.attach(parent, c).unwrap();
    let s = run(&f, parent, 800.0, 600.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    let bc = s.boxed(c).unwrap();
    assert!(near(ba.y, 0.0));
    assert!(near(bb.y, 40.0));
    assert!(near(bc.y, 65.0));
    // Auto-width children fill parent's content width (200, parent
    // has zero padding so content == border-box width).
    assert!(near(ba.w, 200.0));
    assert!(near(bb.w, 200.0));
    assert!(near(bc.w, 200.0));
    // Parent's auto height = total child stack.
    let bp = s.boxed(parent).unwrap();
    assert!(near(bp.h, 80.0));
}

#[test]
fn margin_offsets_box_position() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(50.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(20.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(15.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let root = f.insert(None, style);
    let s = run(&f, root, 400.0, 300.0);
    let b = s.boxed(root).unwrap();
    assert!(near(b.x, 20.0));
    assert!(near(b.y, 15.0));
}

#[test]
fn absolute_pinned_to_padding_box() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        padding: Rect::splat(LengthPercentage::Px(8.0)),
        size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(200.0)),
        ..StyleData::default()
    };
    let abs_style = StyleData {
        position: Position::Absolute,
        size: StyleSize::new(Dimension::Px(30.0), Dimension::Px(30.0)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let abs = f.insert(None, abs_style);
    f.attach(parent, abs).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let ba = s.boxed(abs).unwrap();
    // Absolute child pinned to parent's content (padding-inside) origin.
    assert!(near(ba.x, 8.0));
    assert!(near(ba.y, 8.0));
}

#[test]
fn border_offsets_child_origin() {
    let mut f = LayoutForest::new();
    let parent_style = StyleData {
        size: StyleSize::new(Dimension::Px(200.0), Dimension::Px(200.0)),
        border: Rect::splat(LengthPercentage::Px(4.0)),
        padding: Rect::splat(LengthPercentage::Px(6.0)),
        ..StyleData::default()
    };
    let child_style = StyleData {
        size: StyleSize::new(Dimension::Px(30.0), Dimension::Px(15.0)),
        ..StyleData::default()
    };
    let parent = f.insert(None, parent_style);
    let child = f.insert(None, child_style);
    f.attach(parent, child).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let bc = s.boxed(child).unwrap();
    // Child origin = border (4) + padding (6) = 10 on each axis.
    assert!(near(bc.x, 10.0));
    assert!(near(bc.y, 10.0));
}

#[test]
fn negative_margin_pulls_sibling_up() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let a_style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(40.0)),
        ..StyleData::default()
    };
    let b_style = StyleData {
        size: StyleSize::new(Dimension::Px(50.0), Dimension::Px(40.0)),
        margin: Rect {
            left: LengthPercentageAuto::Px(0.0),
            right: LengthPercentageAuto::Px(0.0),
            top: LengthPercentageAuto::Px(-10.0),
            bottom: LengthPercentageAuto::Px(0.0),
        },
        ..StyleData::default()
    };
    let a = f.insert(None, a_style);
    let b = f.insert(None, b_style);
    f.attach(parent, a).unwrap();
    f.attach(parent, b).unwrap();
    let s = run(&f, parent, 400.0, 300.0);
    let ba = s.boxed(a).unwrap();
    let bb = s.boxed(b).unwrap();
    // Collapse of (0, -10) = -10 → b sits 10 above a's bottom edge.
    assert!(near(bb.y, ba.y + ba.h - 10.0));
}

#[test]
fn solver_writes_root_and_child_boxes_through_forest() {
    let mut f = LayoutForest::new();
    let root = f.insert(None, StyleData::default());
    let child = f.insert(None, StyleData::default());
    f.attach(root, child).unwrap();
    let s = run(&f, root, 100.0, 100.0);
    assert!(s.boxed(root).is_some());
    assert!(s.boxed(child).is_some());
}
