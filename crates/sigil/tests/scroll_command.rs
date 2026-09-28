#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::hal::{Point, ScrollEventInit};
use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use sigil_kittest::FixedMeasure;
use widget_support::compute;

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 60.0,
    height: 16.0,
};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

#[test]
fn scroll_viewport_has_scroll_view_role() {
    sigil::reactive::Runtime::scope("sigil-scroll-role", || {
        let mut cx = Cx::new_root();
        let content = container();
        let root = cx.add(scroll(content, 800.0));
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::ScrollView);
    });
}

#[test]
fn scroll_emits_viewport_and_scrollbar_rects() {
    sigil::reactive::Runtime::scope("sigil-scroll-rects", || {
        let mut cx = Cx::new_root();
        let content = container();
        let root = cx.add(scroll(content, 800.0));
        compute(&mut cx, root, VIEWPORT, Box::new(TEXT_MEASURE));

        let scene = emit_scene(&cx, root);
        let rect_count = scene
            .commands
            .iter()
            .filter(|c| matches!(c, SceneCommand::Rect(_)))
            .count();
        assert!(
            rect_count >= 2,
            "expected viewport + scrollbar track rects, got {rect_count}"
        );
    });
}

#[test]
fn scroll_at_zero_has_no_thumb_offset() {
    sigil::reactive::Runtime::scope("sigil-scroll-zero-offset", || {
        let mut cx = Cx::new_root();
        let content = container();
        let root = cx.add(scroll(content, 800.0));
        compute(&mut cx, root, VIEWPORT, Box::new(TEXT_MEASURE));

        let scene = emit_scene(&cx, root);
        let rects: Vec<_> = scene
            .commands
            .iter()
            .filter_map(|c| match c {
                SceneCommand::Rect(r) => Some(r),
                _ => None,
            })
            .collect();
        assert!(!rects.is_empty(), "should emit at least one rect");
        let first = rects.first().expect("rects is non-empty");
        assert!(
            first.rect.origin.y < 1.0,
            "viewport rect origin.y should be ~0 when scroll=0, got {}",
            first.rect.origin.y
        );
    });
}

#[test]
fn scroll_thumb_proportional_to_content() {
    sigil::reactive::Runtime::scope("sigil-scroll-thumb-size", || {
        let mut cx = Cx::new_root();
        let content = container();
        let root = cx.add(scroll(content, 480.0).height(240.0));
        compute(&mut cx, root, VIEWPORT, Box::new(TEXT_MEASURE));

        let scene = emit_scene(&cx, root);
        let rect_heights: Vec<f32> = scene
            .commands
            .iter()
            .filter_map(|c| match c {
                SceneCommand::Rect(r) => Some(r.rect.height),
                _ => None,
            })
            .collect();
        assert!(
            rect_heights.iter().any(|&h| h > 0.0 && h < 240.0),
            "scrollbar thumb height should be partial viewport height"
        );
    });
}

#[test]
fn scroll_thumb_no_offset_at_zero_with_overflow() {
    sigil::reactive::Runtime::scope("sigil-scroll-thumb-zero", || {
        let mut cx = Cx::new_root();
        let content = container();
        let root = cx.add(scroll(content, 800.0).height(240.0));
        compute(&mut cx, root, VIEWPORT, Box::new(TEXT_MEASURE));
        let scene = emit_scene(&cx, root);
        let rect_count = scene
            .commands
            .iter()
            .filter(|c| matches!(c, SceneCommand::Rect(_)))
            .count();
        assert!(rect_count >= 2);
    });
}

#[test]
fn scroll_fill_uses_measured_parent_height_for_bounds() {
    sigil::reactive::Runtime::scope("sigil-scroll-fill-height", || {
        let mut cx = Cx::new_root();
        let viewport = scroll(container(), 800.0).fill();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(500.0, 360.0))
                .child(viewport),
        );
        let scroll_id = cx.nth_child(root, 0).expect("scroll viewport exists");
        let body_id = cx.nth_child(scroll_id, 0).expect("scroll body exists");
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("initial layout computes");

        let viewport_height = layout
            .result(scroll_id)
            .expect("scroll layout")
            .rect()
            .height;
        assert!(
            (viewport_height - 360.0).abs() < 1.0,
            "fill viewport must inherit the 360px parent height, got {viewport_height}"
        );

        EventRouter::new().route(
            &mut cx,
            &layout,
            ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
                position: Point::new(250.0, 180.0),
                delta: ScrollDelta::Pixels(Point::new(0.0, -1_000.0)),
                modifiers: ModifiersState::empty(),
                phase: ScrollPhase::Moved,
            })),
        );
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("scrolled layout computes");

        let body_y = layout.result(body_id).expect("body layout").rect().origin.y;
        assert!(
            (body_y + 440.0).abs() < 1.0,
            "800px content in a measured 360px viewport must clamp at -440px, got {body_y}"
        );
    });
}

#[test]
fn scroll_can_follow_growth_without_overriding_manual_position() {
    sigil::reactive::Runtime::scope("sigil-scroll-follow-bottom", || {
        let extent = signal(800.0_f32);
        let offset = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let _root = cx.add(
            scroll(container(), extent)
                .height(240.0)
                .scroll_bind(offset)
                .scroll_to_end()
                .auto_pin_to_bottom(true),
        );
        assert_eq!(offset.get(), 560.0);

        extent.set(1_000.0);
        assert_eq!(offset.get(), 760.0);

        offset.set(300.0);
        extent.set(1_200.0);
        assert_eq!(offset.get(), 300.0);
    });
}
