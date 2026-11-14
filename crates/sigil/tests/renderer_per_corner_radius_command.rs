#![cfg(feature = "kittest")]

use crate::scene_support;

use scene_support::color;
use sigil::geometry::{Point, Rect};
use sigil::renderer::{WgpuRenderer, WgpuSurface};
use sigil::scene::{
    CommandScene, CornerRadii, DrawGradient, DrawRect, GradientStop, LinearGradient, SceneBuilder,
    SceneCommand, Stroke, StrokeAlign, StrokeStyle,
};
use sigil::style::{Color, Rgba};
use smallvec::smallvec;

#[test]
fn per_corner_radii_round_trip_with_strokes() {
    let left_round = CornerRadii::new(8.0, 0.0, 0.0, 8.0);
    let stroke = solid_stroke();
    let mut builder = SceneBuilder::new();
    builder.draw_rect(
        DrawRect::new(rect(0.0, 0.0), color(24, 32, 48, 255))
            .border_radius(left_round)
            .stroked(stroke),
    );
    builder.draw_gradient(
        DrawGradient::new(rect(56.0, 0.0), gradient())
            .border_radius(CornerRadii::top(10.0))
            .opacity(9_000)
            .stroked(stroke),
    );
    builder.draw_rect(DrawRect::new(rect(112.0, 0.0), color(8, 12, 20, 255)));
    let scene = builder.finish();

    assert!(matches!(
        scene.commands(),
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
    ] = scene.commands()
    else {
        assert_eq!(scene.len(), 3);
        return;
    };
    assert_eq!(first.border_radius_raw(), left_round);
    assert_eq!(first.stroke(), Some(stroke));
    assert_eq!(second.border_radius_raw(), CornerRadii::top(10.0));
    assert_eq!(second.stroke(), Some(stroke));
    assert_eq!(third.border_radius_raw(), CornerRadii::ZERO);
    assert_eq!(third.stroke(), None);

    let render_path: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = render_path;
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
        Point::new(104.0, 48.0),
        smallvec![
            GradientStop::new(0.0, color(20, 90, 180, 255)),
            GradientStop::new(1.0, color(220, 80, 140, 255)),
        ],
    )
}

const fn solid_stroke() -> Stroke {
    Stroke::new(
        Color {
            r: 230,
            g: 230,
            b: 242,
            a: 255,
        },
        2.0,
        StrokeStyle::Solid,
        StrokeAlign::Centered,
    )
}

const fn rect(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 48.0, 48.0)
}
