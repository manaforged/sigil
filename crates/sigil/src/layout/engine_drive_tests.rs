use super::*;
use crate::geometry::{LogicalSize, Size as HalSize};
use crate::kernel::TextContent;
use crate::layout::style_data::{Dimension, Size as StyleSize, StyleData};

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

fn rect(d: &LayoutDrive, node: NodeHandle) -> (f32, f32, f32, f32) {
    let b = d.computed_box(node).expect("node reached");
    (b.x, b.y, b.w, b.h)
}

#[test]
fn block_roots_resolve_to_viewport_or_fixed_size() {
    let mut d = drive();
    let root = d.insert(StyleData::default());
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
    assert_eq!(rect(&d, root), (0.0, 0.0, 640.0, 0.0));
    let fixed_root = d.insert(fixed(120.0, 80.0));
    d.measure(fixed_root, StyleSize::new(640.0, 480.0)).unwrap();
    assert_eq!(rect(&d, fixed_root), (0.0, 0.0, 120.0, 80.0));
}

#[test]
fn block_child_is_measured_idempotently_and_orphans_are_unreached() {
    let mut d = drive();
    let root = d.insert(fixed(200.0, 200.0));
    let child = d.insert(fixed(50.0, 40.0));
    let orphan = d.insert(fixed(10.0, 10.0));
    d.attach(root, child).unwrap();
    let viewport = StyleSize::new(400.0, 400.0);
    d.measure(root, viewport).unwrap();
    assert_eq!(rect(&d, root), (0.0, 0.0, 200.0, 200.0));
    assert_eq!(rect(&d, child), (0.0, 0.0, 50.0, 40.0));
    assert!(d.computed_box(orphan).is_none());
    d.measure(root, viewport).unwrap();
    assert_eq!(rect(&d, root), (0.0, 0.0, 200.0, 200.0));
    assert_eq!(rect(&d, child), (0.0, 0.0, 50.0, 40.0));
}

#[test]
fn flex_root_dispatches_through_flex_solver() {
    let mut d = drive();
    let root = d.insert(StyleData {
        display: crate::layout::style_data::Display::Flex,
        ..fixed(300.0, 100.0)
    });
    let a = d.insert(fixed(50.0, 50.0));
    let b = d.insert(fixed(80.0, 50.0));
    d.attach(root, a).unwrap();
    d.attach(root, b).unwrap();
    d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
    assert_eq!((rect(&d, root).2, rect(&d, root).3), (300.0, 100.0));
    assert_eq!((rect(&d, a).0, rect(&d, a).2), (0.0, 50.0));
    assert_eq!((rect(&d, b).0, rect(&d, b).2), (50.0, 80.0));
}

#[test]
fn stale_root_handle_errors() {
    let id = drive().insert(StyleData::default());
    assert!(drive().measure(id, StyleSize::new(10.0, 10.0)).is_err());
}
