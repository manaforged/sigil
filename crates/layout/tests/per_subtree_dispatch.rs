//! Q1 tertiary — regression suite for per-subtree solver dispatch.
//!
//! Before Q1 tertiary, the engine driver picked one solver at the
//! layout root and that solver recursed the entire tree. CSS interleaves
//! block-in-flex and flex-in-block freely, so a `display: flex` widget
//! nested inside a `display: block` parent (or vice versa) was being
//! laid out by the wrong algorithm.
//!
//! These tests build minimal interleaved trees through the public
//! `LayoutDrive` surface and assert that each child subtree was solved
//! by the formatting context its own `display` selects.
//!
//! Coverage:
//!   1. `flex_inside_block_lays_out_as_flex` — row-flex children get
//!      distinct horizontal positions (the block solver would stack
//!      them vertically and leave x identical).
//!   2. `block_inside_flex_lays_out_as_block` — block children get
//!      distinct vertical positions (the flex solver would lay them
//!      along the parent's row main axis).
//!   3. `nested_block_flex_block_alternation` — three-deep `block >
//!      flex_row > block_column > text` alternation places the deepest
//!      leaf at the expected coordinate.
//!   4. `nested_flex_block_flex_alternation` — symmetric `flex_column >
//!      block > flex_row > text` chain.
//!   5. `flex_column_with_block_display_child_computes_correct_intrinsic_height`
//!      — Q1-primary interaction: the hypothetical-pass dispatch in
//!      `container_intrinsic_main` MUST also branch on display so a
//!      block-display child reports its real block-stacked height back
//!      to the flex parent. Without patch 1 the parent under-measures.

#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::panic,
    clippy::similar_names
)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

use sigil_hal::{LogicalSize, Size as HalSize};
use sigil_kernel::TextContent;
use sigil_layout::{
    Dimension, LayoutDrive, Size as StyleSize, StyleData, StyleDisplay as Display,
    StyleFlexDirection as FlexDirection, TextMeasure,
};

/// Stub measurer — every glyph is 8x16 logical px. Matches the shape
/// the existing `tests/stub/mod.rs` uses so test text leaves have a
/// non-zero intrinsic size to drive parent sizing.
struct StubText;

impl TextMeasure for StubText {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextContent, _space: HalSize) -> LogicalSize {
        let chars = match source {
            TextContent::Static(v) => v.chars().count() as f32,
            TextContent::Reactive(_) => 0.0,
            TextContent::Compute(build) => build().chars().count() as f32,
        };
        LogicalSize::new(chars * 8.0, 16.0)
    }
}

fn drive() -> LayoutDrive {
    LayoutDrive::new(Box::new(StubText))
}

fn block_style() -> StyleData {
    StyleData {
        display: Display::Block,
        ..StyleData::default()
    }
}

fn flex_style(dir: FlexDirection) -> StyleData {
    StyleData {
        display: Display::Flex,
        flex_direction: dir,
        ..StyleData::default()
    }
}

fn fixed(w: f32, h: f32) -> StyleData {
    StyleData {
        size: StyleSize::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
}

fn fixed_block(w: f32, h: f32) -> StyleData {
    let mut s = fixed(w, h);
    s.display = Display::Block;
    s
}

/// Root block parent contains a flex row; the flex row's three children
/// must end up at distinct x positions (50, 80, 110 — fixed widths of
/// 30 px laid out horizontally starting at x=50 because the flex parent
/// sits inside a block parent at x=50). If the per-subtree dispatch is
/// missing, the block solver continues recursing and stacks the three
/// children vertically (all at x=50, distinct y).
#[test]
fn flex_inside_block_lays_out_as_flex() {
    let mut d = drive();
    let root = d.insert(fixed_block(400.0, 200.0));
    // Inner flex_row with fixed size, placed at the block's content
    // origin (0, 0 since root has no padding).
    let row = d.insert({
        let mut s = flex_style(FlexDirection::Row);
        s.size = StyleSize::new(Dimension::Px(300.0), Dimension::Px(40.0));
        s
    });
    let a = d.insert(fixed(30.0, 30.0));
    let b = d.insert(fixed(30.0, 30.0));
    let c = d.insert(fixed(30.0, 30.0));
    d.attach(root, row).unwrap();
    d.attach(row, a).unwrap();
    d.attach(row, b).unwrap();
    d.attach(row, c).unwrap();
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();

    let ab = d.computed_box(a).expect("a reached");
    let bb = d.computed_box(b).expect("b reached");
    let cb = d.computed_box(c).expect("c reached");

    assert_eq!(ab.x, 0.0, "first row child at flex origin x");
    assert_eq!(bb.x, 30.0, "second child at x = 30 (flex row main axis)");
    assert_eq!(cb.x, 60.0, "third child at x = 60");
    // All three at the same y — confirms flex row (not block stacking).
    assert_eq!(ab.y, bb.y, "row children share y");
    assert_eq!(bb.y, cb.y, "row children share y");
}

/// Root flex column contains a block child; that block child must stack
/// its three children vertically (distinct y). If the per-subtree
/// dispatch is missing, the flex solver continues recursing into the
/// block child and tries to lay out the three grandchildren along the
/// column's main axis with cross-stretch, producing identical y values.
#[test]
fn block_inside_flex_lays_out_as_block() {
    let mut d = drive();
    let root = d.insert({
        let mut s = flex_style(FlexDirection::Column);
        s.size = StyleSize::new(Dimension::Px(400.0), Dimension::Px(400.0));
        s
    });
    let inner_block = d.insert(fixed_block(300.0, 300.0));
    let a = d.insert(fixed_block(80.0, 20.0));
    let b = d.insert(fixed_block(80.0, 20.0));
    let c = d.insert(fixed_block(80.0, 20.0));
    d.attach(root, inner_block).unwrap();
    d.attach(inner_block, a).unwrap();
    d.attach(inner_block, b).unwrap();
    d.attach(inner_block, c).unwrap();
    d.measure(root, StyleSize::new(640.0, 640.0)).unwrap();

    let ab = d.computed_box(a).expect("a reached");
    let bb = d.computed_box(b).expect("b reached");
    let cb = d.computed_box(c).expect("c reached");

    assert!(
        ab.y < bb.y && bb.y < cb.y,
        "block stacking: y must be strictly ascending, got {} / {} / {}",
        ab.y,
        bb.y,
        cb.y
    );
    assert_eq!(bb.y - ab.y, 20.0, "second stacks after first's height");
    assert_eq!(cb.y - bb.y, 20.0, "third stacks after second's height");
    // x must be identical — block doesn't fan out on the inline axis
    // for fixed-width children.
    assert_eq!(ab.x, bb.x, "block children share x");
    assert_eq!(bb.x, cb.x, "block children share x");
}

/// `block > flex_row > block_column > text` — three-level alternation.
/// Verifies the dispatch fires on every level, not just the first hop.
#[test]
fn nested_block_flex_block_alternation() {
    let mut d = drive();
    let root = d.insert(fixed_block(500.0, 500.0));
    let row = d.insert({
        let mut s = flex_style(FlexDirection::Row);
        s.size = StyleSize::new(Dimension::Px(400.0), Dimension::Px(200.0));
        s
    });
    // Two `block` flex items, each holding a stack of two block children.
    let col_a = d.insert(fixed_block(100.0, 200.0));
    let col_b = d.insert(fixed_block(100.0, 200.0));
    let leaf_a1 = d.insert(fixed_block(60.0, 30.0));
    let leaf_a2 = d.insert(fixed_block(60.0, 30.0));
    let leaf_b1 = d.insert(fixed_block(60.0, 30.0));
    let leaf_b2 = d.insert(fixed_block(60.0, 30.0));
    d.attach(root, row).unwrap();
    d.attach(row, col_a).unwrap();
    d.attach(row, col_b).unwrap();
    d.attach(col_a, leaf_a1).unwrap();
    d.attach(col_a, leaf_a2).unwrap();
    d.attach(col_b, leaf_b1).unwrap();
    d.attach(col_b, leaf_b2).unwrap();
    d.measure(root, StyleSize::new(800.0, 800.0)).unwrap();

    let a1 = d.computed_box(leaf_a1).unwrap();
    let a2 = d.computed_box(leaf_a2).unwrap();
    let b1 = d.computed_box(leaf_b1).unwrap();
    let b2 = d.computed_box(leaf_b2).unwrap();

    // Within column a: block stacking, vertical advance of 30.
    assert_eq!(a2.y - a1.y, 30.0, "col_a leaves stack vertically");
    assert_eq!(a1.x, a2.x, "col_a leaves share x");
    // Within column b: same.
    assert_eq!(b2.y - b1.y, 30.0, "col_b leaves stack vertically");
    assert_eq!(b1.x, b2.x, "col_b leaves share x");
    // Across columns: col_b is to the right of col_a (flex row).
    assert!(
        b1.x > a1.x,
        "col_b is right of col_a (flex row): {} > {}",
        b1.x,
        a1.x
    );
}

/// `flex_column > block > flex_row > leaves` — symmetric to the prior
/// test but the alternation starts with flex at the root.
#[test]
fn nested_flex_block_flex_alternation() {
    let mut d = drive();
    let root = d.insert({
        let mut s = flex_style(FlexDirection::Column);
        s.size = StyleSize::new(Dimension::Px(500.0), Dimension::Px(500.0));
        s
    });
    let mid_block = d.insert(fixed_block(400.0, 200.0));
    let inner_row = d.insert({
        let mut s = flex_style(FlexDirection::Row);
        s.size = StyleSize::new(Dimension::Px(300.0), Dimension::Px(60.0));
        s
    });
    let leaf_x = d.insert(fixed(40.0, 40.0));
    let leaf_y = d.insert(fixed(40.0, 40.0));
    let leaf_z = d.insert(fixed(40.0, 40.0));
    d.attach(root, mid_block).unwrap();
    d.attach(mid_block, inner_row).unwrap();
    d.attach(inner_row, leaf_x).unwrap();
    d.attach(inner_row, leaf_y).unwrap();
    d.attach(inner_row, leaf_z).unwrap();
    d.measure(root, StyleSize::new(800.0, 800.0)).unwrap();

    let bx = d.computed_box(leaf_x).unwrap();
    let by = d.computed_box(leaf_y).unwrap();
    let bz = d.computed_box(leaf_z).unwrap();

    // inner_row dispatched as flex → distinct x, shared y.
    assert_eq!(bx.x, 0.0, "first row leaf at row origin x");
    assert_eq!(by.x, 40.0, "second row leaf at x = 40");
    assert_eq!(bz.x, 80.0, "third row leaf at x = 80");
    assert_eq!(bx.y, by.y, "row leaves share y");
    assert_eq!(by.y, bz.y, "row leaves share y");
}

/// Q1 primary interaction — the hypothetical-pass dispatch in
/// `container_intrinsic_main` must also branch on display. A flex
/// column with a block-display child of auto-main size needs to measure
/// the block child's natural height by RUNNING the block stacking
/// algorithm against the child's grandchildren — not the flex
/// algorithm. Without the patch-1 dispatch, the hypothetical pass uses
/// flex layout and the parent reports the wrong main size.
///
/// Tree: `flex_column` parent (auto height) → block child (auto height)
/// → three fixed 60x25 block grandchildren. The expected parent height
/// is 75 (3 × 25) plus any chrome (zero here).
#[test]
fn flex_column_with_block_display_child_computes_correct_intrinsic_height() {
    let mut d = drive();
    let root = d.insert({
        // flex column, auto height, fixed width so the cross axis is
        // constrained but main is content-driven.
        let mut s = flex_style(FlexDirection::Column);
        s.size = StyleSize::new(Dimension::Px(400.0), Dimension::Auto);
        s
    });
    let block_child = d.insert(block_style()); // both axes auto
    let leaf_a = d.insert(fixed_block(60.0, 25.0));
    let leaf_b = d.insert(fixed_block(60.0, 25.0));
    let leaf_c = d.insert(fixed_block(60.0, 25.0));
    d.attach(root, block_child).unwrap();
    d.attach(block_child, leaf_a).unwrap();
    d.attach(block_child, leaf_b).unwrap();
    d.attach(block_child, leaf_c).unwrap();
    d.measure(root, StyleSize::new(800.0, 800.0)).unwrap();

    let rb = d.computed_box(root).expect("root reached");
    let bb = d.computed_box(block_child).expect("block_child reached");
    let a = d.computed_box(leaf_a).unwrap();
    let b = d.computed_box(leaf_b).unwrap();
    let c = d.computed_box(leaf_c).unwrap();

    // The block child must stack vertically (verifies patch 2 ran).
    assert_eq!(b.y - a.y, 25.0, "block leaves stack — patch 2 active");
    assert_eq!(c.y - b.y, 25.0, "block leaves stack — patch 2 active");

    // The hypothetical pass during the parent's intrinsic main
    // computation MUST have used the block algorithm, so the parent's
    // height tracks the block-stacked total (75 px), not whatever flex
    // would have produced for the same tree.
    assert_eq!(
        bb.h, 75.0,
        "block child auto height = sum of leaves = 75, got {}",
        bb.h
    );
    assert!(
        rb.h >= 75.0,
        "flex column parent absorbs block child's natural height: {} >= 75",
        rb.h
    );
}
