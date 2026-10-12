#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

use crate::geometry::{PhysicalSize, Point, Rect};
use crate::style::Color;
use crate::style::Rgba8;

use super::*;
use crate::scene::{DrawRect, SceneCommand, SceneRectStyle};

const RED: Rgba8 = Rgba8 {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let px = pixmap.pixel(x, y).unwrap();
    [px.red(), px.green(), px.blue(), px.alpha()]
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(Point::new(x, y), w, h)
}

const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);

fn draw(
    commands: Vec<SceneCommand>,
    clear: Color,
    viewport: (f32, f32),
    size: (u32, u32),
) -> (Pixmap, CpuRenderer) {
    let mut renderer = CpuRenderer::new();
    let scene = CommandScene { commands };
    let out = renderer
        .render(&scene, clear, viewport, PhysicalSize::new(size.0, size.1))
        .unwrap();
    (out, renderer)
}

fn solid(rect: Rect) -> SceneCommand {
    SceneCommand::Rect(DrawRect::new(rect, SceneRectStyle::solid(RED)))
}

#[test]
fn clear_color_fills_the_target() {
    let (out, _) = draw(vec![], Color::rgba(0.0, 0.0, 1.0, 1.0), (4.0, 4.0), (4, 4));
    assert_eq!(pixel(&out, 0, 0), [0, 0, 255, 255]);
}

#[test]
fn rect_lands_at_physical_scale() {
    let commands = vec![solid(rect(1.0, 1.0, 2.0, 2.0))];
    let (out, renderer) = draw(commands, WHITE, (4.0, 4.0), (8, 8));
    assert_eq!(pixel(&out, 3, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&out, 1, 1), [255, 255, 255, 255]);
    assert_eq!(pixel(&out, 6, 6), [255, 255, 255, 255]);
    assert_eq!(renderer.last_frame_stats().rectangle_instances, 1);
}

#[test]
fn clip_limits_a_rect_to_the_pushed_bounds() {
    let commands = vec![
        SceneCommand::PushClip(rect(0.0, 0.0, 2.0, 4.0)),
        solid(rect(0.0, 0.0, 4.0, 4.0)),
        SceneCommand::PopClip,
    ];
    let (out, _) = draw(commands, WHITE, (4.0, 4.0), (4, 4));
    assert_eq!(pixel(&out, 1, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(&out, 3, 2), [255, 255, 255, 255]);
}

#[test]
fn transform_moves_the_rect() {
    let commands = vec![
        SceneCommand::PushTransform([[1.0, 0.0, 2.0], [0.0, 1.0, 0.0]]),
        solid(rect(0.0, 0.0, 1.0, 4.0)),
        SceneCommand::PopTransform,
    ];
    let (out, _) = draw(commands, WHITE, (4.0, 4.0), (4, 4));
    assert_eq!(pixel(&out, 2, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&out, 0, 0), [255, 255, 255, 255]);
}

#[test]
fn opacity_layer_scales_alpha() {
    let commands = vec![
        SceneCommand::SaveLayer { opacity: 0.5 },
        solid(rect(0.0, 0.0, 4.0, 4.0)),
        SceneCommand::RestoreLayer,
    ];
    let (out, _) = draw(commands, WHITE, (4.0, 4.0), (4, 4));
    let [r, g, b, _] = pixel(&out, 1, 1);
    assert_eq!(r, 255);
    assert!((180..=195).contains(&g), "g={g}");
    assert!((180..=195).contains(&b), "b={b}");
}

#[test]
fn glyph_run_paints_ink_inside_its_rect() {
    use crate::style::TextStyle;
    let run = crate::scene::shape_glyph_run(
        rect(2.0, 2.0, 60.0, 30.0),
        "III".to_owned(),
        RED,
        TextStyle::default(),
        10_000,
    );
    let commands = vec![SceneCommand::GlyphRun(
        crate::renderer::scene_command::DrawGlyphRun::new(run),
    )];
    let (out, renderer) = draw(commands, WHITE, (64.0, 34.0), (128, 68));
    let ink = out
        .pixels()
        .iter()
        .filter(|p| p.red() > 0 && p.green() < 128)
        .count();
    assert!(ink > 20, "expected glyph ink, found {ink} red pixels");
    assert_eq!(pixel(&out, 0, 0), [255, 255, 255, 255]);
    assert_eq!(renderer.last_frame_stats().glyph_runs, 1);
}

#[test]
fn blur_spreads_a_point_without_changing_total_alpha_much() {
    let mut pixmap = Pixmap::new(9, 9).unwrap();
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    pixmap.fill_rect(
        tiny_skia::Rect::from_xywh(4.0, 4.0, 1.0, 1.0).unwrap(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );
    shadow::blur(&mut pixmap, 1.0);
    assert!(pixmap.pixel(4, 4).unwrap().alpha() < 255);
    assert!(pixmap.pixel(3, 4).unwrap().alpha() > 0);
    assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);
}
