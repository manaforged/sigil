#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use shell as _;
use skrifa as _;
use smallvec as _;
use text as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use icons as _;

use hal::Size;
use kernel::{TextContent, prelude::*};
use layout::{LayoutBuilderExt, LayoutEngine, LayoutResult, LayoutStyle, TextMeasure};
use renderer::{
    CommandScene, DrawGlyphRun, DrawRect, SceneCommand, emit_scene,
    unsupported_style_warning_count_for,
};
use style::TextLineHeight;
use style::{FocusRing, NodeStyleExt, Theme};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _source: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::default()
    }
}

#[test]
fn theme_accent_rect_emits_in_scene() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .class("bg-accent"),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let rect = first_rect(&scene).unwrap();

    assert_eq!(rect.node, Some(root));
    assert_eq!(rect.rect.width, 120.0);
    assert_eq!(rect.rect.height, 48.0);
    assert_eq!(rect.color, Theme::default().accent);
}

#[test]
fn text_emits_default_on_surface_color() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Readable"));

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Readable").unwrap();

    assert_eq!(text.run.node, Some(root));
    assert_eq!(text.run.text, "Readable");
    assert_eq!(text.run.color, Theme::default().on_surface);
}

#[test]
fn styled_text_color_overrides_default() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Accent").class("text-accent"));

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Accent").unwrap();

    assert_eq!(text.run.color, Theme::default().accent);
}

#[test]
fn styled_text_typography_emits_in_scene() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Large").class("text-xl font-bold leading-8"));

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Large").unwrap();

    assert_eq!(text.run.style.font_size, 20.0);
    assert_eq!(text.run.style.font_weight, 700);
    assert_eq!(text.run.style.line_height, TextLineHeight::Absolute(32.0));
}

#[test]
fn visible_focus_ring_emits_four_rects() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
            .focus_ring(FocusRing::static_visible(true)),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let rects = rects_for_node(&scene, root);

    assert_eq!(rects.len(), 4);
    let first = rects.first().unwrap();
    assert_eq!(first.rect.origin.x, -4.0);
    assert_eq!(first.rect.origin.y, -4.0);
    assert_eq!(first.rect.width, 108.0);
    assert_eq!(first.rect.height, 2.0);
    assert_eq!(first.color, Theme::default().accent);
}

#[test]
fn nested_text_scene_rect_is_absolute() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(240.0, 120.0).padding(24.0))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(120.0, 60.0).padding(8.0))
                    .child(text("Nested")),
            ),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let text = find_text(&scene, "Nested").unwrap();

    assert_eq!(text.run.rect.origin.x, 32.0);
    assert_eq!(text.run.rect.origin.y, 32.0);
}

#[test]
fn unsupported_shadow_warns_once_across_frames() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(32.0, 32.0))
            .class("shadow-xl"),
    );

    layout(&mut cx, root);

    for _frame in 0..60 {
        let _scene = emit_scene(&cx, root);
    }

    assert_eq!(unsupported_style_warning_count_for("shadow-xl", root), 1);
}

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    engine
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout compute succeeds");
    assert!(
        cx.get(root)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .is_some()
    );
}

fn first_rect(scene: &CommandScene) -> Option<&DrawRect> {
    scene.commands.iter().find_map(|command| match command {
        SceneCommand::Rect(rect) => Some(rect),
        _ => None,
    })
}

fn rects_for_node(scene: &CommandScene, node: NodeId) -> Vec<&DrawRect> {
    scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) if rect.node == Some(node) => Some(rect),
            _ => None,
        })
        .collect()
}

fn find_text<'a>(scene: &'a CommandScene, expected: &str) -> Option<&'a DrawGlyphRun> {
    scene.find_text(expected)
}
