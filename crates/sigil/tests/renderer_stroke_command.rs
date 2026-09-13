#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil::hal::{Point, Rect};
use sigil::renderer::{
    CornerRadii, GradientStop, LinearGradient, SceneBuilder, SceneCommand, SceneGradientStyle,
    SceneRectStyle, Stroke, StrokeAlign, StrokeStyle,
};
use sigil::style::Rgba8;
use smallvec::smallvec;

#[test]
fn stroke_constructs_with_public_fields() {
    let stroke = solid_stroke(StrokeAlign::Centered, 2.0);

    assert_eq!(
        stroke.color,
        Rgba8 {
            r: 26,
            g: 51,
            b: 77,
            a: 230,
        }
    );
    assert!((stroke.width - 2.0).abs() < 1e-6);
    assert_eq!(stroke.style, StrokeStyle::Solid);
    assert_eq!(stroke.align, StrokeAlign::Centered);
}

#[test]
fn with_stroke_builder_round_trips_rect_and_gradient_styles() {
    let rect_stroke = solid_stroke(StrokeAlign::Centered, 2.0);
    let gradient_stroke = dashed_stroke(StrokeAlign::Inside, 4.0, 6.0);
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect(0.0, 0.0), rect_style().with_stroke(rect_stroke));
    builder.draw_gradient(
        rect(56.0, 0.0),
        SceneGradientStyle::new(gradient(), CornerRadii::uniform(8.0), 9_000)
            .with_stroke(gradient_stroke),
    );
    let scene = builder.finish();

    let [SceneCommand::Rect(rect), SceneCommand::Gradient(gradient)] = scene.commands.as_slice()
    else {
        unreachable!("scene should contain one rect and one gradient");
    };
    assert_eq!(rect.stroke, Some(rect_stroke));
    assert_eq!(gradient.stroke, Some(gradient_stroke));
}

#[test]
fn stroke_commands_cover_solid_dashed_outside_and_none() {
    let outside = solid_stroke(StrokeAlign::Outside, 5.0);
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect(0.0, 0.0), rect_style());
    builder.draw_rect(
        rect(56.0, 0.0),
        rect_style().with_stroke(dashed_stroke(StrokeAlign::Inside, 3.0, 7.0)),
    );
    builder.draw_gradient(
        rect(112.0, 0.0),
        SceneGradientStyle::new(gradient(), CornerRadii::uniform(10.0), 10_000)
            .with_stroke(outside),
    );
    let scene = builder.finish();

    let [
        SceneCommand::Rect(zero),
        SceneCommand::Rect(dashed),
        SceneCommand::Gradient(grad),
    ] = scene.commands.as_slice()
    else {
        unreachable!("scene should contain three stroke cases");
    };
    assert_eq!(zero.stroke, None);
    assert_eq!(
        dashed.stroke,
        Some(dashed_stroke(StrokeAlign::Inside, 3.0, 7.0))
    );
    assert_eq!(grad.stroke, Some(outside));
}

const fn rect_style() -> SceneRectStyle {
    SceneRectStyle::new(color(8, 12, 20, 255), CornerRadii::uniform(8.0), 10_000)
}

const fn solid_stroke(align: StrokeAlign, width: f32) -> Stroke {
    Stroke {
        color: Rgba8 {
            r: 26,
            g: 51,
            b: 77,
            a: 230,
        },
        width,
        style: StrokeStyle::Solid,
        align,
    }
}

const fn dashed_stroke(align: StrokeAlign, width: f32, gap: f32) -> Stroke {
    Stroke {
        color: Rgba8 {
            r: 179,
            g: 153,
            b: 51,
            a: 255,
        },
        width,
        style: StrokeStyle::Dashed(gap),
        align,
    }
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

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
