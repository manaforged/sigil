#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]

use sigil::layout::{Dimension, Display, LayoutDrive, NodeHandle, ResolvedBox, Size, StyleData};
use sigil_kittest::FixedMeasure;

fn drive() -> LayoutDrive {
    LayoutDrive::new(Box::new(FixedMeasure::default()))
}

fn fixed(w: f32, h: f32) -> StyleData {
    StyleData {
        size: Size::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
}

fn flex_fixed(w: f32, h: f32) -> StyleData {
    let mut s = fixed(w, h);
    s.display = Display::Flex;
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
    let mut d = drive();
    let root = d.insert(StyleData::default());
    let middle = d.insert(flex_fixed(200.0, 100.0));
    let leaf_a = d.insert(fixed(40.0, 30.0));
    let leaf_b = d.insert(fixed(40.0, 30.0));
    d.attach(root, middle).unwrap();
    d.attach(middle, leaf_a).unwrap();
    d.attach(middle, leaf_b).unwrap();

    d.measure(root, Size::new(400.0, 300.0)).unwrap();

    assert!(d.computed_box(root).is_some());
    assert!(d.computed_box(middle).is_some());
    assert!(d.computed_box(leaf_a).is_some());
    assert!(d.computed_box(leaf_b).is_some());

    let rb = box_of(&d, root);
    assert!(near(rb.w, 400.0));
    assert!(near(rb.h, 100.0));
}

#[test]
fn flex_root_dispatch_routes_through_flex_solver() {
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
