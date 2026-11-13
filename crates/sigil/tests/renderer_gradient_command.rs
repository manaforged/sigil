#![cfg(feature = "kittest")]
#![allow(clippy::unwrap_used, clippy::panic)]

use crate::scene_support;

use scene_support::{color, rect};
use sigil::geometry::Point;
use sigil::renderer::{WgpuRenderer, WgpuSurface};
use sigil::scene::{
    CommandScene, CornerRadii, DrawGradient, DrawRect, Gradient, GradientStop, LinearGradient,
    PaintFill, SceneBuilder, SceneCommand,
};
use sigil::style::Rgba;
use smallvec::smallvec;

#[test]
fn command_scene_accepts_rect_and_gradient_draws() {
    let gradient = gradient();
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect_style(rect(0.0, 0.0, 48.0, 48.0)));
    builder.draw_gradient(
        DrawGradient::new(rect(56.0, 0.0, 96.0, 48.0), gradient.clone())
            .border_radius(CornerRadii::uniform(8.0))
            .opacity(8_500),
    );
    let scene = builder.finish();

    assert!(matches!(
        scene.commands(),
        [SceneCommand::Rect(_), SceneCommand::Gradient(_)]
    ));
    let [_, SceneCommand::Gradient(draw)] = scene.commands() else {
        panic!("expected gradient draw");
    };
    assert_eq!(*draw.gradient(), Gradient::from(gradient));

    let method: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = method;
}

#[test]
fn fill_variants_cover_solid_and_linear_gradient() {
    let solid = PaintFill::Solid(color(24, 48, 72, 255));
    let linear = PaintFill::LinearGradient(gradient());

    assert!(matches!(solid, PaintFill::Solid(_)));
    assert!(matches!(linear, PaintFill::LinearGradient(_)));
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Rgba,
    (f32, f32),
) -> Result<(), sigil::Error>;

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

const fn rect_style(rect: sigil::geometry::Rect) -> DrawRect {
    DrawRect::new(rect, color(8, 12, 20, 255)).border_radius(CornerRadii::uniform(8.0))
}
