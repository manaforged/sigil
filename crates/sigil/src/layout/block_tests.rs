#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use super::*;
use crate::geometry::{LogicalSize, Size as HalSize};
use crate::kernel::TextContent;
use crate::layout::forest::{AvailableSpace, LayoutForest, LocalId, MeasureScope};
use crate::layout::style_data::{
    BoxSizing, Dimension, Display, LengthPercentage, LengthPercentageAuto, Position, Rect,
    Size as StyleSize, StyleData,
};

struct NullText;
impl crate::layout::TextMeasure for NullText {
    fn measure(&self, _: &TextContent, _: HalSize) -> LogicalSize {
        LogicalSize::default()
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.001
}

fn run(forest: &LayoutForest, root: LocalId, w: f32, h: f32) -> BlockSolver<'_> {
    let measurer: &'static NullText = &NullText;
    let scope = MeasureScope::new(forest, measurer, AvailableSpace::definite(w, h));
    let solver = BlockSolver::new(scope);
    solver.solve(forest.refer(root).unwrap()).unwrap();
    solver
}

fn sized(w: Dimension, h: Dimension) -> StyleData {
    StyleData {
        size: StyleSize::new(w, h),
        ..StyleData::default()
    }
}

fn px(w: f32, h: f32) -> StyleData {
    sized(Dimension::Px(w), Dimension::Px(h))
}

fn margin(left: f32, top: f32, bottom: f32) -> Rect<LengthPercentageAuto> {
    Rect {
        left: LengthPercentageAuto::Px(left),
        right: LengthPercentageAuto::Px(0.0),
        top: LengthPercentageAuto::Px(top),
        bottom: LengthPercentageAuto::Px(bottom),
    }
}

fn kid(f: &mut LayoutForest, parent: LocalId, style: StyleData) -> LocalId {
    let id = f.insert(None, style);
    f.attach(parent, id).unwrap();
    id
}

fn sibling_gap(top_margin: f32, bottom_margin: f32) -> f32 {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, sized(Dimension::Px(200.0), Dimension::Auto));
    let a = kid(
        &mut f,
        parent,
        StyleData {
            margin: margin(0.0, 0.0, bottom_margin),
            ..px(50.0, 30.0)
        },
    );
    let b = kid(
        &mut f,
        parent,
        StyleData {
            margin: margin(0.0, top_margin, 0.0),
            ..px(50.0, 30.0)
        },
    );
    let s = run(&f, parent, 400.0, 300.0);
    let (ba, bb) = (s.boxed(a).unwrap(), s.boxed(b).unwrap());
    assert!(near(ba.y, 0.0));
    bb.y - ba.y - ba.h
}

#[test]
fn explicit_width_and_height_are_honoured() {
    let mut f = LayoutForest::new();
    let root = f.insert(None, px(150.0, 60.0));
    let b = run(&f, root, 400.0, 300.0).boxed(root).unwrap();
    assert!(near(b.w, 150.0) && near(b.h, 60.0) && near(b.x, 0.0) && near(b.y, 0.0));
}

#[test]
fn auto_width_fills_containing_block() {
    let mut f = LayoutForest::new();
    let root = f.insert(None, StyleData::default());
    assert!(near(
        run(&f, root, 800.0, 600.0).boxed(root).unwrap().w,
        800.0
    ));
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
    assert!(near(
        run(&f, root, 200.0, 150.0).boxed(root).unwrap().h,
        16.0
    ));
}

#[test]
fn padding_and_border_offset_child_origin() {
    for (border, padding) in [(0.0, 10.0), (4.0, 6.0)] {
        let mut f = LayoutForest::new();
        let parent = f.insert(
            None,
            StyleData {
                border: Rect::splat(LengthPercentage::Px(border)),
                padding: Rect::splat(LengthPercentage::Px(padding)),
                ..px(200.0, 200.0)
            },
        );
        let child = kid(&mut f, parent, px(40.0, 20.0));
        let cb = run(&f, parent, 400.0, 300.0).boxed(child).unwrap();
        assert!(near(cb.x, 10.0) && near(cb.y, 10.0));
    }
}

#[test]
fn sibling_margins_collapse_or_sum() {
    assert!(near(sibling_gap(12.0, 20.0), 20.0));
    assert!(near(sibling_gap(-5.0, 15.0), 10.0));
    assert!(near(sibling_gap(-10.0, 0.0), -10.0));
}

#[test]
fn absolute_child_does_not_advance_flow() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let abs = kid(
        &mut f,
        parent,
        StyleData {
            position: Position::Absolute,
            ..px(40.0, 40.0)
        },
    );
    let flow = kid(&mut f, parent, px(60.0, 20.0));
    let s = run(&f, parent, 400.0, 300.0);
    assert!(near(s.boxed(flow).unwrap().y, 0.0));
    let ba = s.boxed(abs).unwrap();
    assert!(near(ba.w, 40.0) && near(ba.h, 40.0));
}

#[test]
fn display_none_child_is_skipped() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let hidden = kid(
        &mut f,
        parent,
        StyleData {
            display: Display::None,
            ..px(99.0, 99.0)
        },
    );
    let visible = kid(&mut f, parent, px(20.0, 10.0));
    let s = run(&f, parent, 400.0, 300.0);
    assert!(s.boxed(hidden).is_none());
    assert!(near(s.boxed(visible).unwrap().y, 0.0));
}

#[test]
fn nested_blocks_stack_vertically() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, sized(Dimension::Px(200.0), Dimension::Auto));
    let kids =
        [40.0, 25.0, 15.0].map(|h| kid(&mut f, parent, sized(Dimension::Auto, Dimension::Px(h))));
    let s = run(&f, parent, 800.0, 600.0);
    for (id, y) in kids.into_iter().zip([0.0, 40.0, 65.0]) {
        let b = s.boxed(id).unwrap();
        assert!(near(b.y, y) && near(b.w, 200.0));
    }
    assert!(near(s.boxed(parent).unwrap().h, 80.0));
}

#[test]
fn margin_offsets_box_position() {
    let mut f = LayoutForest::new();
    let root = f.insert(
        None,
        StyleData {
            margin: margin(20.0, 15.0, 0.0),
            ..px(50.0, 50.0)
        },
    );
    let b = run(&f, root, 400.0, 300.0).boxed(root).unwrap();
    assert!(near(b.x, 20.0) && near(b.y, 15.0));
}

#[test]
fn absolute_pinned_to_padding_box() {
    let mut f = LayoutForest::new();
    let parent = f.insert(
        None,
        StyleData {
            padding: Rect::splat(LengthPercentage::Px(8.0)),
            ..px(200.0, 200.0)
        },
    );
    let abs = kid(
        &mut f,
        parent,
        StyleData {
            position: Position::Absolute,
            ..px(30.0, 30.0)
        },
    );
    let ba = run(&f, parent, 400.0, 300.0).boxed(abs).unwrap();
    assert!(near(ba.x, 8.0) && near(ba.y, 8.0));
}

#[test]
fn solver_writes_root_and_child_boxes_through_forest() {
    let mut f = LayoutForest::new();
    let root = f.insert(None, StyleData::default());
    let child = kid(&mut f, root, StyleData::default());
    let s = run(&f, root, 100.0, 100.0);
    assert!(s.boxed(root).is_some() && s.boxed(child).is_some());
}

#[path = "block_tests/sizing.rs"]
mod sizing;
