#![allow(unused_crate_dependencies)]
//! W1.5 — `inset` placement for `Position::Absolute` children.
//!
//! Verifies that the block + flex solvers read `style.inset` when
//! placing an absolutely positioned child and pick the trailing edge
//! when only the trailing inset is fixed. Existing "all-Auto pins to
//! padding origin" behavior is also regression-tested.

#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

mod stub;

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_kernel as _;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_layout::{LengthPercentageAuto, Rect as StyleRect};
use sigil_reactive as _;
use sigil_style as _;
use slotmap as _;
use smallvec as _;
use taffy as _;

use stub::stub_engine;

const PARENT_W: f32 = 200.0;
const PARENT_H: f32 = 120.0;
const CHILD_W: f32 = 40.0;
const CHILD_H: f32 = 30.0;

#[test]
fn block_absolute_top_left_px_inset_offsets_from_padding_origin() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute)
                        .with_layout(|s| {
                            s.inset = StyleRect {
                                left: LengthPercentageAuto::Px(30.0),
                                right: LengthPercentageAuto::Auto,
                                top: LengthPercentageAuto::Px(20.0),
                                bottom: LengthPercentageAuto::Auto,
                            };
                        }),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 30.0);
    assert_eq!(rect.origin.y, 20.0);
    assert_eq!(rect.width, CHILD_W);
    assert_eq!(rect.height, CHILD_H);
}

#[test]
fn block_absolute_bottom_inset_anchors_against_trailing_edge() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute)
                        .with_layout(|s| {
                            s.inset = StyleRect {
                                left: LengthPercentageAuto::Px(15.0),
                                right: LengthPercentageAuto::Auto,
                                top: LengthPercentageAuto::Auto,
                                bottom: LengthPercentageAuto::Px(10.0),
                            };
                        }),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    // x: leading edge (Px(15)) wins.
    assert_eq!(rect.origin.x, 15.0);
    // y: leading is Auto, trailing Px(10) → parent_h - child_h - 10.
    assert_eq!(rect.origin.y, PARENT_H - CHILD_H - 10.0);
}

#[test]
fn block_absolute_right_inset_with_auto_left_anchors_against_right_edge() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute)
                        .with_layout(|s| {
                            s.inset = StyleRect {
                                left: LengthPercentageAuto::Auto,
                                right: LengthPercentageAuto::Px(25.0),
                                top: LengthPercentageAuto::Px(5.0),
                                bottom: LengthPercentageAuto::Auto,
                            };
                        }),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    // x: leading Auto, trailing Px(25) → parent_w - child_w - 25.
    assert_eq!(rect.origin.x, PARENT_W - CHILD_W - 25.0);
    assert_eq!(rect.origin.y, 5.0);
}

#[test]
fn flex_absolute_inset_top_left_offsets_from_padding_origin() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute)
                        .with_layout(|s| {
                            s.inset = StyleRect {
                                left: LengthPercentageAuto::Px(40.0),
                                right: LengthPercentageAuto::Auto,
                                top: LengthPercentageAuto::Px(12.0),
                                bottom: LengthPercentageAuto::Auto,
                            };
                        }),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 40.0);
    assert_eq!(rect.origin.y, 12.0);
}

#[test]
fn flex_absolute_bottom_right_inset_anchors_against_trailing_edges() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute)
                        .with_layout(|s| {
                            s.inset = StyleRect {
                                left: LengthPercentageAuto::Auto,
                                right: LengthPercentageAuto::Px(8.0),
                                top: LengthPercentageAuto::Auto,
                                bottom: LengthPercentageAuto::Px(6.0),
                            };
                        }),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, PARENT_W - CHILD_W - 8.0);
    assert_eq!(rect.origin.y, PARENT_H - CHILD_H - 6.0);
}

#[test]
fn block_absolute_all_auto_inset_keeps_padding_origin_fallback() {
    // Regression: default-Auto inset must keep the historical pin-to-
    // padding-origin behavior so widgets that never set `inset` are
    // unchanged by the W1.5 cutover.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .fixed_size(CHILD_W, CHILD_H)
                        .position(Position::Absolute),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 0.0);
    assert_eq!(rect.origin.y, 0.0);
}

fn first_child(cx: &Cx, root: NodeId) -> NodeId {
    cx.get(root).unwrap().children().first().copied().unwrap()
}

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}
