use super::*;
use crate::style_data::{Dimension, Display, Size as StyleSize, StyleData};
use sigil_hal::{LogicalSize, Size as HalSize};
use sigil_kernel::TextContent;

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
        size: StyleSize::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
}

fn flex_fixed(w: f32, h: f32) -> StyleData {
    let mut s = fixed(w, h);
    s.display = Display::Flex;
    s
}

#[test]
fn single_block_root_resolves_to_viewport() {
    let mut d = drive();
    let root = d.insert(StyleData::default());
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
    let box_root = d.computed_box(root).expect("root reached");
    assert_eq!(box_root.w, 640.0);
    assert_eq!(box_root.h, 0.0_f32.max(0.0)); // auto height with no children
    assert_eq!(box_root.x, 0.0);
    assert_eq!(box_root.y, 0.0);
}

#[test]
fn single_block_root_with_fixed_size() {
    let mut d = drive();
    let root = d.insert(fixed(120.0, 80.0));
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
    let b = d.computed_box(root).unwrap();
    assert_eq!(b.w, 120.0);
    assert_eq!(b.h, 80.0);
}

#[test]
fn block_root_with_block_child_writes_both_boxes() {
    let mut d = drive();
    let root = d.insert(fixed(200.0, 200.0));
    let child = d.insert(fixed(50.0, 40.0));
    d.attach(root, child).unwrap();
    d.measure(root, StyleSize::new(400.0, 400.0)).unwrap();
    let rb = d.computed_box(root).unwrap();
    let cb = d.computed_box(child).unwrap();
    assert_eq!(rb.w, 200.0);
    assert_eq!(rb.h, 200.0);
    assert_eq!(cb.w, 50.0);
    assert_eq!(cb.h, 40.0);
    // Child placed at parent border-box origin (no padding).
    assert_eq!(cb.x, 0.0);
    assert_eq!(cb.y, 0.0);
}

#[test]
fn flex_root_dispatches_through_flex_solver() {
    let mut d = drive();
    let root = d.insert(flex_fixed(300.0, 100.0));
    let a = d.insert(fixed(50.0, 50.0));
    let b = d.insert(fixed(80.0, 50.0));
    d.attach(root, a).unwrap();
    d.attach(root, b).unwrap();
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
    let rb = d.computed_box(root).unwrap();
    assert_eq!(rb.w, 300.0);
    assert_eq!(rb.h, 100.0);
    let ab = d.computed_box(a).unwrap();
    let bb = d.computed_box(b).unwrap();
    // Flex Row direction: items lay out horizontally. First item
    // at x=0, second item at x = 50.
    assert_eq!(ab.x, 0.0);
    assert_eq!(bb.x, 50.0);
    assert_eq!(ab.w, 50.0);
    assert_eq!(bb.w, 80.0);
}

#[test]
fn computed_box_for_unreached_handle_returns_none() {
    let mut d = drive();
    let root = d.insert(fixed(40.0, 40.0));
    let orphan = d.insert(fixed(10.0, 10.0));
    d.measure(root, StyleSize::new(100.0, 100.0)).unwrap();
    assert!(d.computed_box(root).is_some());
    // Orphan not attached anywhere — never reached by the walk.
    assert!(d.computed_box(orphan).is_none());
}

#[test]
fn measure_is_idempotent_under_same_inputs() {
    let mut d = drive();
    let root = d.insert(fixed(64.0, 48.0));
    let child = d.insert(fixed(16.0, 16.0));
    d.attach(root, child).unwrap();
    let viewport = StyleSize::new(200.0, 200.0);
    d.measure(root, viewport).unwrap();
    let first_root = d.computed_box(root).unwrap();
    let first_child = d.computed_box(child).unwrap();
    d.measure(root, viewport).unwrap();
    assert_eq!(d.computed_box(root).unwrap(), first_root);
    assert_eq!(d.computed_box(child).unwrap(), first_child);
}

#[test]
fn stale_root_handle_errors() {
    let drive_a = drive();
    // Synthesize a handle the driver did not mint by reusing one
    // from an empty drive — its inner LocalId resolves to no node.
    let mut drive_b = LayoutDrive::new(Box::new(NullText));
    let id = drive_b.insert(StyleData::default());
    drop(drive_a);
    // Different LayoutDrive than the one that minted the id.
    let mut other = LayoutDrive::new(Box::new(NullText));
    let err = other.measure(id, StyleSize::new(10.0, 10.0));
    assert!(err.is_err());
}
