#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, GradientStop, LinearGradient, RendererError, SceneBuilder,
    SceneCommand, SceneGradientStyle, SceneRectStyle, Shadow, ShadowKind, WgpuRenderer,
    WgpuSurface,
};
use sigil_style::Rgba8;
use smallvec::smallvec;

#[test]
fn shadow_constructs_with_public_fields() {
    let shadow = key_shadow();

    assert_array_close(shadow.color, [0.0, 0.0, 0.0, 0.28]);
    assert_pair_close(shadow.offset, [0.0, 4.0]);
    assert!((shadow.blur - 12.0).abs() < 1e-6);
    assert!((shadow.spread - 1.0).abs() < 1e-6);
    assert_eq!(shadow.kind, ShadowKind::Outer);
}

#[test]
fn shadow_constructor_defaults_to_outer_and_builder_sets_inner() {
    let shadow = Shadow::new([1.0, 2.0], 3.0, 4.0, [0.1, 0.2, 0.3, 0.4]);

    assert_eq!(shadow.kind, ShadowKind::Outer);
    assert_eq!(shadow.with_kind(ShadowKind::Inner).kind, ShadowKind::Inner);
}

#[test]
fn shadow_command_stacks_before_casting_rect() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(16.0), CornerRadii::uniform(12.0), ambient_shadow());
    builder.draw_shadow(rect(16.0), CornerRadii::uniform(12.0), key_shadow());
    builder.draw_rect(
        rect(16.0),
        SceneRectStyle::new(
            color(250, 252, 255, 255),
            CornerRadii::uniform(12.0),
            10_000,
        ),
    );
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::DrawShadow { .. },
            SceneCommand::DrawShadow { .. },
            SceneCommand::Rect(_),
        ]
    ));
}

#[test]
fn shadow_commands_cover_solid_gradient_and_asymmetric_radii() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(0.0), CornerRadii::uniform(10.0), key_shadow());
    builder.draw_rect(
        rect(0.0),
        SceneRectStyle::new(
            color(235, 240, 248, 255),
            CornerRadii::uniform(10.0),
            10_000,
        ),
    );
    builder.draw_shadow(rect(72.0), CornerRadii::top(16.0), ambient_shadow());
    builder.draw_gradient(
        rect(72.0),
        SceneGradientStyle::new(gradient(), CornerRadii::top(16.0), 10_000),
    );
    let scene = builder.finish();

    assert_eq!(scene.commands.len(), 4);
    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::DrawShadow { .. },
            SceneCommand::Rect(_),
            SceneCommand::DrawShadow { .. },
            SceneCommand::Gradient(_),
        ]
    ));

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
        Point::new(72.0, 0.0),
        Point::new(128.0, 56.0),
        smallvec![
            GradientStop::new(0.0, color(42, 115, 220, 255)),
            GradientStop::new(1.0, color(80, 184, 140, 255)),
        ],
    )
}

const fn key_shadow() -> Shadow {
    Shadow::new([0.0, 4.0], 12.0, 1.0, [0.0, 0.0, 0.0, 0.28])
}

const fn ambient_shadow() -> Shadow {
    Shadow::new([0.0, 1.0], 6.0, 0.0, [0.0, 0.0, 0.0, 0.16])
}

const fn rect(x: f32) -> Rect {
    Rect::new(Point::new(x, 8.0), 56.0, 56.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn assert_array_close(actual_values: [f32; 4], expected_values: [f32; 4]) {
    for (actual, expected) in actual_values.into_iter().zip(expected_values) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

fn assert_pair_close(actual_values: [f32; 2], expected_values: [f32; 2]) {
    for (actual, expected) in actual_values.into_iter().zip(expected_values) {
        assert!((actual - expected).abs() < 1e-6);
    }
}
