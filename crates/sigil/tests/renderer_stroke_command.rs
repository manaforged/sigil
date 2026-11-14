use crate::scene_support;

use scene_support::color;
use sigil::geometry::{Point, Rect};
use sigil::scene::{
    CornerRadii, DrawGradient, DrawRect, GradientStop, LinearGradient, SceneBuilder, SceneCommand,
    Stroke, StrokeAlign, StrokeStyle,
};
use sigil::style::Color;
use smallvec::smallvec;

#[test]
fn stroke_constructs_with_public_fields() {
    let stroke = solid_stroke(StrokeAlign::Centered, 2.0);

    assert_eq!(
        stroke.color(),
        Color {
            r: 26,
            g: 51,
            b: 77,
            a: 230,
        }
    );
    assert!((stroke.width() - 2.0).abs() < 1e-6);
    assert_eq!(stroke.style(), StrokeStyle::Solid);
    assert_eq!(stroke.align(), StrokeAlign::Centered);
}

#[test]
fn with_stroke_builder_round_trips_rect_and_gradient_styles() {
    let rect_stroke = solid_stroke(StrokeAlign::Centered, 2.0);
    let gradient_stroke = dashed_stroke(StrokeAlign::Inside, 4.0, 6.0);
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect_style(rect(0.0, 0.0)).with_stroke(rect_stroke));
    builder.draw_gradient(
        DrawGradient::new(rect(56.0, 0.0), gradient())
            .border_radius(CornerRadii::uniform(8.0))
            .opacity(9_000)
            .with_stroke(gradient_stroke),
    );
    let scene = builder.finish();

    let [SceneCommand::Rect(rect), SceneCommand::Gradient(gradient)] = scene.commands() else {
        unreachable!("scene should contain one rect and one gradient");
    };
    assert_eq!(rect.stroke(), Some(rect_stroke));
    assert_eq!(gradient.stroke(), Some(gradient_stroke));
}

#[test]
fn stroke_commands_cover_solid_dashed_outside_and_none() {
    let outside = solid_stroke(StrokeAlign::Outside, 5.0);
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect_style(rect(0.0, 0.0)));
    builder.draw_rect(rect_style(rect(56.0, 0.0)).with_stroke(dashed_stroke(
        StrokeAlign::Inside,
        3.0,
        7.0,
    )));
    builder.draw_gradient(
        DrawGradient::new(rect(112.0, 0.0), gradient())
            .border_radius(CornerRadii::uniform(10.0))
            .with_stroke(outside),
    );
    let scene = builder.finish();

    let [
        SceneCommand::Rect(zero),
        SceneCommand::Rect(dashed),
        SceneCommand::Gradient(grad),
    ] = scene.commands()
    else {
        unreachable!("scene should contain three stroke cases");
    };
    assert_eq!(zero.stroke(), None);
    assert_eq!(
        dashed.stroke(),
        Some(dashed_stroke(StrokeAlign::Inside, 3.0, 7.0))
    );
    assert_eq!(grad.stroke(), Some(outside));
}

const fn rect_style(rect: sigil::geometry::Rect) -> DrawRect {
    DrawRect::new(rect, color(8, 12, 20, 255)).border_radius(CornerRadii::uniform(8.0))
}

const fn solid_stroke(align: StrokeAlign, width: f32) -> Stroke {
    Stroke::new(
        Color {
            r: 26,
            g: 51,
            b: 77,
            a: 230,
        },
        width,
        StrokeStyle::Solid,
        align,
    )
}

const fn dashed_stroke(align: StrokeAlign, width: f32, gap: f32) -> Stroke {
    Stroke::new(
        Color {
            r: 179,
            g: 153,
            b: 51,
            a: 255,
        },
        width,
        StrokeStyle::Dashed(gap),
        align,
    )
}

fn gradient() -> LinearGradient {
    LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(48.0, 48.0),
        smallvec![
            GradientStop::new(0.0, color(20, 90, 180, 255)),
            GradientStop::new(1.0, color(220, 80, 140, 255)),
        ],
    )
}

const fn rect(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 48.0, 48.0)
}
