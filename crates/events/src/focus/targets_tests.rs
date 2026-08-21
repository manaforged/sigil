use sigil_hal::Point;
use sigil_kernel::container;
use sigil_layout::prelude::*;

use super::*;
use crate::EventRouter;

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(
        &self,
        _content: &sigil_kernel::TextContent,
        _space: sigil_hal::Size,
    ) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::default()
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

#[test]
fn is_hittable_target_rejects_a_zero_area_node() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(0.0, 0.0)),
            ),
    );
    let target = cx.nth_child(root, 0).expect("zero-size child");
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(100.0, 100.0))
        .expect("layout");
    let focus = FocusManager::default();
    assert!(
        !focus.is_hittable_target(&cx, &layout, target),
        "collapsed (zero-area) node must not accept synthetic clicks"
    );
}

#[test]
fn is_hittable_target_rejects_a_node_outside_the_viewport() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(
                container().focusable(true).layout(
                    LayoutStyle::block()
                        .fixed_size(10.0, 10.0)
                        .position(Position::Absolute)
                        .with_layout(|style| {
                            style.inset.left = LengthPercentageAuto::Px(500.0);
                            style.inset.top = LengthPercentageAuto::Px(500.0);
                        }),
                ),
            ),
    );
    let outside = cx.nth_child(root, 0).expect("child must exist");
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(100.0, 100.0))
        .expect("layout must succeed");
    let router = EventRouter::new();

    assert!(!router.is_click_target(&cx, &layout, outside));
}
