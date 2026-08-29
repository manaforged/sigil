#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Engine-driver integration tests.
//!
//! Exercises the public [`LayoutDrive`] surface end-to-end: build a
//! three-level mixed-display tree, drive `measure`, verify every node
//! has a `ComputedBox` and that dimensions match hand-computed values.
//!
//! Lives outside the crate (under `tests/`) so the test binary depends
//! only on the public API — no `crate::` paths, no `pub(crate)`
//! reach-arounds. The forest/solver internals stay sealed inside the
//! lib crate.

#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

// Workspace shared dev-deps surfaced into the integration target so the
// crate-level `unused-crate-dependencies` lint does not fire on the
// integration test binary.

use sigil_hal::{LogicalSize, Size as HalSize};
use sigil_kernel::TextContent;
use sigil_layout::{
    Dimension, LayoutDrive, NodeHandle, ResolvedBox, Size, StyleData, StyleDisplay, TextMeasure,
};

struct NullText;
impl TextMeasure for NullText {
    fn measure(&self, _: &TextContent, _: HalSize) -> LogicalSize {
        LogicalSize::default()
    }
}

fn drive() -> LayoutDrive {
    LayoutDrive::new(Box::new(NullText))
}

fn fixed(w: f32, h: f32) -> StyleData {
    StyleData {
        size: Size::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
}

fn flex_fixed(w: f32, h: f32) -> StyleData {
    let mut s = fixed(w, h);
    s.display = StyleDisplay::Flex;
    s
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

fn box_of(d: &LayoutDrive, node: NodeHandle) -> ResolvedBox {
    d.computed_box(node)
        .unwrap_or_else(|| panic!("node missing computed box"))
}

fn assert_box(actual: ResolvedBox, expected: ResolvedBox, label: &str) {
    assert!(
        near(actual.x, expected.x)
            && near(actual.y, expected.y)
            && near(actual.w, expected.w)
            && near(actual.h, expected.h),
        "{label}: actual {actual:?} != expected {expected:?}"
    );
}

const fn boxed(x: f32, y: f32, w: f32, h: f32) -> ResolvedBox {
    ResolvedBox { x, y, w, h }
}

#[test]
fn three_level_block_tree_resolves_every_node() {
    // root (block, 300x200)
    //   middle (block, 200x120)
    //     leaf_a (block, 80x40)
    //     leaf_b (block, 60x30)
    let mut d = drive();
    let root = d.insert(fixed(300.0, 200.0));
    let middle = d.insert(fixed(200.0, 120.0));
    let leaf_a = d.insert(fixed(80.0, 40.0));
    let leaf_b = d.insert(fixed(60.0, 30.0));
    d.attach(root, middle).unwrap();
    d.attach(middle, leaf_a).unwrap();
    d.attach(middle, leaf_b).unwrap();

    d.measure(root, Size::new(640.0, 480.0)).unwrap();

    assert_box(box_of(&d, root), boxed(0.0, 0.0, 300.0, 200.0), "root");
    assert_box(box_of(&d, middle), boxed(0.0, 0.0, 200.0, 120.0), "middle");
    assert_box(box_of(&d, leaf_a), boxed(0.0, 0.0, 80.0, 40.0), "leaf_a");
    assert_box(box_of(&d, leaf_b), boxed(0.0, 40.0, 60.0, 30.0), "leaf_b");
}

#[test]
fn three_level_mixed_display_tree_writes_every_box() {
    // root (block) — outer container, auto-sized to viewport
    //   middle (flex container — flex children measured by block solver
    //     since the engine driver dispatches at root display; the
    //     middle simply nests as a block under block-dispatch)
    //     leaf_a (block, 40x30)
    //     leaf_b (block, 40x30)
    let mut d = drive();
    let root = d.insert(StyleData::default());
    let middle = d.insert(flex_fixed(200.0, 100.0));
    let leaf_a = d.insert(fixed(40.0, 30.0));
    let leaf_b = d.insert(fixed(40.0, 30.0));
    d.attach(root, middle).unwrap();
    d.attach(middle, leaf_a).unwrap();
    d.attach(middle, leaf_b).unwrap();

    d.measure(root, Size::new(400.0, 300.0)).unwrap();

    // Brief requires: every reachable node has a non-None ComputedBox.
    assert!(d.computed_box(root).is_some());
    assert!(d.computed_box(middle).is_some());
    assert!(d.computed_box(leaf_a).is_some());
    assert!(d.computed_box(leaf_b).is_some());

    // Root is auto-sized — width fills the viewport, height stacks
    // children. Middle's explicit 200x100 box does not stretch root.
    let rb = box_of(&d, root);
    assert!(near(rb.w, 400.0));
    assert!(near(rb.h, 100.0));
}

#[test]
fn flex_root_dispatch_routes_through_flex_solver() {
    // root (flex Row, 300x60)
    //   leaf_a (50x40)
    //   leaf_b (70x40)
    // Expectation under flex-Row: leaf_a at x=0, leaf_b at x=50.
    let mut d = drive();
    let root = d.insert(flex_fixed(300.0, 60.0));
    let leaf_a = d.insert(fixed(50.0, 40.0));
    let leaf_b = d.insert(fixed(70.0, 40.0));
    d.attach(root, leaf_a).unwrap();
    d.attach(root, leaf_b).unwrap();

    d.measure(root, Size::new(640.0, 480.0)).unwrap();

    let rb = box_of(&d, root);
    let ab = box_of(&d, leaf_a);
    let bb = box_of(&d, leaf_b);

    assert!(near(rb.w, 300.0));
    assert!(near(rb.h, 60.0));
    assert!(near(ab.x, 0.0));
    assert!(near(bb.x, 50.0));
    assert!(near(ab.w, 50.0));
    assert!(near(bb.w, 70.0));
}

#[test]
fn measure_is_idempotent_under_same_inputs() {
    let mut d = drive();
    let root = d.insert(fixed(160.0, 80.0));
    let child = d.insert(fixed(40.0, 20.0));
    d.attach(root, child).unwrap();
    let viewport = Size::new(640.0, 480.0);

    d.measure(root, viewport).unwrap();
    let first_root = box_of(&d, root);
    let first_child = box_of(&d, child);

    d.measure(root, viewport).unwrap();
    assert_eq!(box_of(&d, root), first_root);
    assert_eq!(box_of(&d, child), first_child);
}

#[test]
fn unreached_node_returns_none() {
    let mut d = drive();
    let root = d.insert(fixed(50.0, 50.0));
    let orphan = d.insert(fixed(10.0, 10.0));
    d.measure(root, Size::new(100.0, 100.0)).unwrap();
    assert!(d.computed_box(root).is_some());
    assert!(d.computed_box(orphan).is_none());
}
