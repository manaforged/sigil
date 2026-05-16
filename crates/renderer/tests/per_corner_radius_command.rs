//! Per-corner radius command integration coverage.

#![allow(missing_docs)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal::{Color, Point, Rect};
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style::Rgba8;
use sigil_text as _;
use skrifa as _;
use smallvec::smallvec;
use taffy as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_icons as _;

use sigil_renderer::{
    CommandScene, CornerRadii, GradientStop, LinearGradient, RendererError, SceneBuilder,
    SceneCommand, SceneGradientStyle, SceneRectStyle, Stroke, StrokeAlign, StrokeStyle,
    WgpuRenderer, WgpuSurface,
};

#[test]
fn per_corner_radii_round_trip_with_strokes() {
    let left_round = CornerRadii::new(8.0, 0.0, 0.0, 8.0);
    let stroke = solid_stroke();
    let mut builder = SceneBuilder::new();
    builder.draw_rect(
        rect(0.0, 0.0),
        SceneRectStyle::new(color(24, 32, 48, 255), left_round, 10_000).with_stroke(stroke),
    );
    builder.draw_gradient(
        rect(56.0, 0.0),
        SceneGradientStyle::new(gradient(), CornerRadii::top(10.0), 9_000).with_stroke(stroke),
    );
    builder.draw_rect(
        rect(112.0, 0.0),
        SceneRectStyle::new(color(8, 12, 20, 255), CornerRadii::ZERO, 10_000),
    );
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::Rect(_),
            SceneCommand::Gradient(_),
            SceneCommand::Rect(_),
        ]
    ));
    let [
        SceneCommand::Rect(first),
        SceneCommand::Gradient(second),
        SceneCommand::Rect(third),
    ] = scene.commands.as_slice()
    else {
        assert_eq!(scene.commands.len(), 3);
        return;
    };
    assert_eq!(first.border_radius, left_round);
    assert_eq!(first.stroke, Some(stroke));
    assert_eq!(second.border_radius, CornerRadii::top(10.0));
    assert_eq!(second.stroke, Some(stroke));
    assert_eq!(third.border_radius, CornerRadii::ZERO);
    assert_eq!(third.stroke, None);

    let render_path: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = render_path;
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
        Point::new(104.0, 48.0),
        smallvec![
            GradientStop::new(0.0, color(20, 90, 180, 255)),
            GradientStop::new(1.0, color(220, 80, 140, 255)),
        ],
    )
}

const fn solid_stroke() -> Stroke {
    Stroke {
        color: [0.9, 0.9, 0.95, 1.0],
        width: 2.0,
        style: StrokeStyle::Solid,
        align: StrokeAlign::Centered,
    }
}

const fn rect(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 48.0, 48.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
