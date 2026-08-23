#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! W1.5 — `inset` placement for `Position::Absolute` children.
//!
//! Verifies that the block + flex solvers read `style.inset` when
//! placing an absolutely positioned child and pick the trailing edge
//! when only the trailing inset is fixed. Existing "all-Auto pins to
//! padding origin" behavior is also regression-tested.

#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

mod stub;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;
use sigil_layout::{LengthPercentageAuto, Rect as StyleRect};

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
                        .inset_left(15.0)
                        .inset_bottom(10.0),
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

#[test]
fn flex_absolute_shrink_wrapped_child_anchors_bottom_right() {
    // Regression: a shrink-wrapped (auto-size) absolute child pinned only
    // by trailing insets must anchor its trailing edges against the
    // parent's padding box. Before the fix the auto axes filled the
    // containing block, so `origin + cb - box - inset` placed the child's
    // leading edge at (-16, -16) — fully offscreen. This is the
    // `toast_queue` shape: a flex_column overlay with content-sized
    // children and `inset_bottom`/`inset_right`.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .position(Position::Absolute)
                            .inset_bottom(16.0)
                            .inset_right(16.0),
                    )
                    .child(container().layout(LayoutStyle::default().fixed_size(60.0, 20.0)))
                    .child(container().layout(LayoutStyle::default().fixed_size(40.0, 30.0))),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    // Shrink-to-fit: widest child wide, stacked heights tall.
    assert_eq!(rect.width, 60.0);
    assert_eq!(rect.height, 50.0);
    assert_eq!(rect.origin.x, PARENT_W - 60.0 - 16.0);
    assert_eq!(rect.origin.y, PARENT_H - 50.0 - 16.0);
}

#[test]
fn flex_absolute_shrink_wrapped_text_leaf_anchors_bottom_right() {
    // Same trailing-anchor rule for a childless text leaf: the measured
    // text extent is the content size (stub measure: 8 px/char, 16 px
    // line height — "hey" is 24x16).
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                text("hey").layout(
                    LayoutStyle::default()
                        .position(Position::Absolute)
                        .inset_bottom(16.0)
                        .inset_right(16.0),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.width, 24.0);
    assert_eq!(rect.height, 16.0);
    assert_eq!(rect.origin.x, PARENT_W - 24.0 - 16.0);
    assert_eq!(rect.origin.y, PARENT_H - 16.0 - 16.0);
}

#[test]
fn flex_absolute_toast_queue_shape_anchors_fully_inside() {
    // The exact `toast_queue` widget shape: absolute flex_column with
    // AlignItems::End, gap, trailing insets, holding flex_row children
    // (fixed-size dot + text, padding, gap). The box's far edge must land
    // exactly at `cb - inset`, not past it.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(PARENT_W, PARENT_H))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .position(Position::Absolute)
                            .inset_bottom(16.0)
                            .inset_right(16.0)
                            .gap(8.0)
                            .align_items(AlignItems::End),
                    )
                    .child(
                        container()
                            .layout(
                                LayoutStyle::flex_row()
                                    .align_items(AlignItems::Center)
                                    .gap(8.0)
                                    .padding(12.0),
                            )
                            .child(container().layout(LayoutStyle::default().fixed_size(8.0, 8.0)))
                            .child(text("hey")),
                    ),
            ),
    );
    let queue = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    // Row content: padding 12 + dot 8 + gap 8 + text 24 + padding 12.
    let rect = layout_rect(&cx, queue);
    assert_eq!(rect.width, 64.0);
    // Row height: padding 12 + text line 16 + padding 12.
    assert_eq!(rect.height, 40.0);
    assert_eq!(rect.origin.x, PARENT_W - 64.0 - 16.0);
    assert_eq!(rect.origin.y, PARENT_H - 40.0 - 16.0);
}

#[test]
fn flex_absolute_mixed_leading_trailing_leading_wins() {
    // Sigil's rule (CSS absolute-placement): when both insets on an axis
    // are fixed, the LEADING inset places the box. The leading-anchored
    // auto axis keeps the historical containing-block fill; only the
    // trailing-anchored auto axis shrink-wraps.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .position(Position::Absolute)
                            .inset_left(10.0)
                            .inset_right(16.0)
                            .inset_bottom(8.0),
                    )
                    .child(container().layout(LayoutStyle::default().fixed_size(60.0, 20.0))),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    // x: leading Px(10) wins over trailing Px(16).
    assert_eq!(rect.origin.x, 10.0);
    // y: leading Auto, trailing Px(8) → shrink-wrapped anchor.
    assert_eq!(rect.height, 20.0);
    assert_eq!(rect.origin.y, PARENT_H - 20.0 - 8.0);
}

#[test]
fn flex_absolute_trailing_inset_empty_child_does_not_invert() {
    // A zero-content child shrink-wraps to 0x0 and anchors at
    // `cb - inset` — the trailing math must not invert to a negative
    // origin.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .position(Position::Absolute)
                        .inset_bottom(16.0)
                        .inset_right(16.0),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.width, 0.0);
    assert_eq!(rect.height, 0.0);
    assert_eq!(rect.origin.x, PARENT_W - 16.0);
    assert_eq!(rect.origin.y, PARENT_H - 16.0);
}

#[test]
fn flex_absolute_leading_inset_auto_size_keeps_containing_block_fill() {
    // Regression pin: an auto-size absolute child anchored by LEADING
    // insets (tooltip / dropdown / virtual-list shape) still fills the
    // containing block — only trailing-anchored axes shrink-wrap.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(PARENT_W, PARENT_H))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .position(Position::Absolute)
                            .inset_left(40.0)
                            .inset_top(12.0),
                    )
                    .child(container().layout(LayoutStyle::default().fixed_size(60.0, 20.0))),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(PARENT_W, PARENT_H))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 40.0);
    assert_eq!(rect.origin.y, 12.0);
    // The final box comes from `write_container` inside the subtree pass,
    // which sizes a container child to content even when the placement
    // pass filled the containing block — so only the origin pin matters
    // here, and it is unchanged.
    assert_eq!(rect.width, 60.0);
    assert_eq!(rect.height, 20.0);
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
