#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

use crate::paint_support as support;

use sigil::geometry::Constraint;
use sigil::kernel::*;
use sigil::layout::{LayoutEngine, LayoutResult, LayoutStyle};
use sigil::scene::{SceneCommand, StrokeAlign, StrokeStyle, emit_scene};
use sigil::style::{Paint, Theme, Token};
use sigil::widgets::Widget;
use support::{TEXT_MEASURE, capture, close};

const SIZE: u32 = 64;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::new(Box::new(TEXT_MEASURE));
    engine
        .compute(cx, root, Constraint::fixed(SIZE as f32, SIZE as f32))
        .expect("layout compute");
}

#[test]
fn border_class_emits_inside_stroke_command() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
            .background(Token::Surface)
            .border_width(2)
            .border_color(Token::Accent),
    );
    layout(&mut cx, root);

    let scene = emit_scene(&cx, root, None);
    let stroke = scene
        .commands()
        .iter()
        .find_map(|command| match command {
            SceneCommand::Rect(rect) => rect.stroke_style(),
            _ => None,
        })
        .expect("bordered container must emit a stroked rect");
    assert!((stroke.width - 2.0).abs() < f32::EPSILON);
    assert_eq!(stroke.align, StrokeAlign::Inside);
    assert_eq!(stroke.style, StrokeStyle::Solid);
    assert_eq!(stroke.color, Theme::default().color(Token::Accent));
}

#[test]
fn border_paints_edge_and_leaves_interior() {
    let img = capture(
        || {
            container()
                .layout(LayoutStyle::block().fixed_size(SIZE as f32, SIZE as f32))
                .background(Token::Surface)
                .border_width(4)
                .border_color(Token::Accent)
        },
        [SIZE, SIZE],
    );

    let theme = Theme::default();
    let edge = img.get_pixel(2, SIZE / 2).0;
    let interior = img.get_pixel(SIZE / 2, SIZE / 2).0;
    assert!(
        close(edge[0], edge[1], edge[2], theme.color(Token::Accent), 6),
        "border edge must paint accent, got ({},{},{})",
        edge[0],
        edge[1],
        edge[2],
    );
    assert!(
        close(
            interior[0],
            interior[1],
            interior[2],
            theme.color(Token::Surface),
            6
        ),
        "interior must stay the surface fill, got ({},{},{})",
        interior[0],
        interior[1],
        interior[2],
    );
}

#[test]
fn border_without_background_shows_parent_through_interior() {
    let img = capture(
        || {
            let bordered = container()
                .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
                .border_width(4)
                .border_color(Token::Accent);
            container()
                .layout(LayoutStyle::block().fixed_size(SIZE as f32, SIZE as f32))
                .background(Token::SurfaceHover)
                .child(bordered)
        },
        [SIZE, SIZE],
    );

    let theme = Theme::default();
    let edge = img.get_pixel(2, 24).0;
    let interior = img.get_pixel(24, 24).0;
    assert!(
        close(edge[0], edge[1], edge[2], theme.color(Token::Accent), 6),
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
            theme.color(Token::SurfaceHover),
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
    let mut cx = Cx::new_root();
    let child = container().layout(LayoutStyle::block().fixed_size(8.0, 8.0));
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(48.0, 48.0))
            .border_width(4)
            .child(child),
    );
    layout(&mut cx, root);

    let child_id = cx.get(root).unwrap().children().first().copied().unwrap();
    let rect = cx
        .get(child_id)
        .and_then(|node| node.extension::<LayoutResult>())
        .expect("child layout result")
        .rect();
    assert!(
        (rect.origin.x - 4.0).abs() < f32::EPSILON && (rect.origin.y - 4.0).abs() < f32::EPSILON,
        "border-4 must inset the child content box by 4px, got origin ({}, {})",
        rect.origin.x,
        rect.origin.y,
    );
}
