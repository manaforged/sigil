use crate::geometry::Point;
use crate::geometry::{Rect, Size};
use crate::kernel::container;
use crate::layout::*;

use super::*;
use crate::events::EventRouter;

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(&self, _: &crate::kernel::TextContent, _: Size) -> crate::geometry::LogicalSize {
        crate::geometry::LogicalSize::default()
    }
}

#[test]
fn rects_intersect_detects_overlap_and_disjoint_rects() {
    let a = Rect::new(Point::new(0.0, 0.0), 10.0, 10.0);
    let overlapping = Rect::new(Point::new(5.0, 5.0), 10.0, 10.0);
    let disjoint = Rect::new(Point::new(20.0, 20.0), 10.0, 10.0);
    let touching = Rect::new(Point::new(10.0, 0.0), 10.0, 10.0);

    assert!(rects_intersect(a, overlapping));
    assert!(!rects_intersect(a, disjoint));
    assert!(
        !rects_intersect(a, touching),
        "touching edges do not overlap"
    );
}

#[test]
fn document_root_walks_to_the_topmost_ancestor() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0))),
    );
    let child = cx.nth_child(root, 0).expect("child must exist");

    assert_eq!(document_root(&cx, root), root);
    assert_eq!(document_root(&cx, child), root);
}

fn laid_out(child: crate::kernel::NodeBuilder) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(child),
    );
    let target = cx.nth_child(root, 0).expect("child must exist");
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(100.0, 100.0))
        .expect("layout must succeed");
    (cx, target, layout)
}

#[test]
fn is_hittable_target_rejects_a_zero_area_node() {
    let (cx, target, layout) = laid_out(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(0.0, 0.0)),
    );
    assert!(
        !FocusManager::default().is_hittable_target(&cx, &layout, target),
        "collapsed (zero-area) node must not accept synthetic clicks"
    );
}

#[test]
fn is_hittable_target_rejects_a_node_outside_the_viewport() {
    let (cx, outside, layout) = laid_out(
        container().focusable(true).layout(
            LayoutStyle::block()
                .fixed_size(10.0, 10.0)
                .position(Position::Absolute)
                .with_layout(|style| {
                    style.inset.left = LengthPercentageAuto::Px(500.0);
                    style.inset.top = LengthPercentageAuto::Px(500.0);
                }),
        ),
    );
    assert!(!EventRouter::new().is_click_target(&cx, &layout, outside));
}
