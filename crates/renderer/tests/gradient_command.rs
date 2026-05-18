//! Gradient command integration coverage.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec::smallvec;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, Fill, GradientStop, LinearGradient, RendererError, SceneBuilder,
    SceneCommand, SceneGradientStyle, SceneRectStyle, WgpuRenderer, WgpuSurface,
};
use sigil_style::Rgba8;

#[test]
fn command_scene_accepts_rect_and_gradient_draws() {
    let gradient = gradient();
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect(0.0, 0.0, 48.0, 48.0), rect_style());
    builder.draw_gradient(
        rect(56.0, 0.0, 96.0, 48.0),
        SceneGradientStyle::new(gradient.clone(), CornerRadii::uniform(8.0), 8_500),
    );
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [SceneCommand::Rect(_), SceneCommand::Gradient(_)]
    ));
    let [_, SceneCommand::Gradient(draw)] = scene.commands.as_slice() else {
        panic!("expected gradient draw");
    };
    assert_eq!(draw.gradient, gradient);

    let method: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = method;
}

#[test]
fn fill_variants_cover_solid_and_linear_gradient() {
    let solid = Fill::Solid(color(24, 48, 72, 255));
    let linear = Fill::LinearGradient(gradient());

    assert!(matches!(solid, Fill::Solid(_)));
    assert!(matches!(linear, Fill::LinearGradient(_)));
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Color,
    (f32, f32),
) -> Result<(), RendererError>;

fn gradient() -> LinearGradient {
    LinearGradient::new(
        Point::new(56.0, 0.0),
        Point::new(152.0, 48.0),
        smallvec![
            GradientStop::new(0.0, color(20, 90, 180, 255)),
            GradientStop::new(1.0, color(220, 80, 140, 255)),
        ],
    )
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn rect_style() -> SceneRectStyle {
    SceneRectStyle::new(color(8, 12, 20, 255), CornerRadii::uniform(8.0), 10_000)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
