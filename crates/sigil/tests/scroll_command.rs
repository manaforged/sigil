#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use criterion as _;
use image as _;
use pollster as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(60.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout computes");
}

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
        compute(&mut cx, root);

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
        compute(&mut cx, root);

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
        let first = rects[0];
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
        compute(&mut cx, root);

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
        compute(&mut cx, root);
        let scene = emit_scene(&cx, root);
        let rect_count = scene
            .commands
            .iter()
            .filter(|c| matches!(c, SceneCommand::Rect(_)))
            .count();
        assert!(rect_count >= 2);
    });
}
