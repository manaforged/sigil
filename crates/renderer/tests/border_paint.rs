#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! DR-057 D3 border wiring — command emission + real offscreen pixels.
//!
//! `border-*` utility classes flipped from DR-056 rejection to painting
//! through the pre-existing R2 `Stroke` primitive. These tests pin the
//! full pipe: class → atom → `RenderStyle` → inside-aligned stroke on the
//! node's rect → actual ink at the border edge, with the interior
//! untouched.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

#[path = "child_paint_repro/support.rs"]
#[allow(dead_code)]
mod support;

use sigil_hal::Size;
use sigil_kernel::{Cx, prelude::*};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutResult, LayoutStyle};
use sigil_renderer::{SceneCommand, StrokeAlign, StrokeStyle, emit_scene};
use sigil_style::{NodeStyleExt, Theme};
use support::{StubMeasure, capture, close};

const SIZE: u32 = 64;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    engine
        .compute(cx, root, Size::fixed(SIZE as f32, SIZE as f32))
        .expect("layout compute");
}

#[test]
fn border_class_emits_inside_stroke_command() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
            .class("bg-surface border-2 border-accent"),
    );
    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let stroke = scene
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Rect(rect) => rect.stroke,
            _ => None,
        })
        .expect("bordered container must emit a stroked rect");
    assert!((stroke.width - 2.0).abs() < f32::EPSILON);
    assert_eq!(stroke.align, StrokeAlign::Inside);
    assert_eq!(stroke.style, StrokeStyle::Solid);
    assert_eq!(stroke.color, Theme::default().accent);
}

#[test]
fn border_paints_edge_and_leaves_interior() {
    let img = capture(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(SIZE as f32, SIZE as f32))
                    .class("bg-surface border-4 border-accent"),
            )
        },
        [SIZE, SIZE],
    );

    let theme = Theme::default();
    let edge = img.get_pixel(2, SIZE / 2).0;
    let interior = img.get_pixel(SIZE / 2, SIZE / 2).0;
    assert!(
        close(edge[0], edge[1], edge[2], theme.accent, 6),
        "border edge must paint accent, got ({},{},{})",
        edge[0],
        edge[1],
        edge[2],
    );
    assert!(
        close(interior[0], interior[1], interior[2], theme.surface, 6),
        "interior must stay the surface fill, got ({},{},{})",
        interior[0],
        interior[1],
        interior[2],
    );
}

#[test]
fn border_without_background_shows_parent_through_interior() {
    let img = capture(
        |cx| {
            let bordered = container()
                .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
                .class("border-4 border-accent");
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(SIZE as f32, SIZE as f32))
                    .class("bg-surface-hover")
                    .child(bordered),
            )
        },
        [SIZE, SIZE],
    );

    let theme = Theme::default();
    let edge = img.get_pixel(2, 24).0;
    let interior = img.get_pixel(24, 24).0;
    assert!(
        close(edge[0], edge[1], edge[2], theme.accent, 6),
        "edge must paint accent, got ({},{},{})",
        edge[0],
        edge[1],
        edge[2],
    );
    assert!(
        close(
            interior[0],
            interior[1],
            interior[2],
            theme.surface_hover,
            6
        ),
        "borderless interior must show the parent surface through the \
         transparent fill, got ({},{},{})",
        interior[0],
        interior[1],
        interior[2],
    );
}

#[test]
fn border_width_insets_child_content_box_model() {
    // `border` is box-model-affecting (DR-057 D3): the solvers reserve
    // the edge, so a child's content origin shifts by the border width
    // exactly like CSS `box-sizing: border-box`.
    let mut cx = Cx::new_root();
    let child = container().layout(LayoutStyle::block().fixed_size(8.0, 8.0));
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
            .class("border-4")
            .child(child),
    );
    layout(&mut cx, root);

    let child_id = cx.get(root).unwrap().children().first().copied().unwrap();
    let rect = cx
        .get(child_id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("child layout result")
        .rect();
    assert!(
        (rect.origin.x - 4.0).abs() < f32::EPSILON && (rect.origin.y - 4.0).abs() < f32::EPSILON,
        "border-4 must inset the child content box by 4px, got origin ({}, {})",
        rect.origin.x,
        rect.origin.y,
    );
}
