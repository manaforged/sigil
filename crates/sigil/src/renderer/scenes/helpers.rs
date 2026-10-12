use crate::geometry::{Point, Rect};
use crate::style::{Color, Rgba8};
use smallvec::smallvec;

use crate::scene::{
    CornerRadii, GradientStop, LinearGradient, PaintFill, PatternKind, SceneBuilder,
    SceneGradientStyle, ScenePatternStyle, SceneRectStyle, Shadow, Stroke, StrokeAlign,
    StrokeStyle,
};

pub(super) fn draw_fill(scene: &mut SceneBuilder, rect: Rect, fill: PaintFill) {
    match fill {
        PaintFill::Solid(color) => scene.draw_rect(
            rect,
            SceneRectStyle::new(color, CornerRadii::uniform(16.0), 10_000),
        ),
        PaintFill::LinearGradient(gradient) => scene.draw_gradient(
            rect,
            SceneGradientStyle::new(gradient, CornerRadii::uniform(16.0), 10_000),
        ),
        PaintFill::Pattern(pattern) => scene.draw_pattern(
            rect,
            ScenePatternStyle::new(pattern, CornerRadii::uniform(16.0), 10_000),
        ),
    }
}

pub(super) const fn slash_kind(color: Rgba8, period: f32, line_width: f32) -> PatternKind {
    PatternKind::Slash {
        color,
        period,
        line_width,
    }
}

pub(super) const DARK: Color = Color::rgba(0.05, 0.06, 0.09, 1.0);

pub(super) fn linear_gradient(
    (x0, y0): (f32, f32),
    (x1, y1): (f32, f32),
    stops: &[(f32, Rgba8)],
) -> PaintFill {
    let stops: Vec<GradientStop> = stops
        .iter()
        .map(|&(offset, color)| GradientStop::new(offset, color))
        .collect();
    PaintFill::LinearGradient(LinearGradient::new(
        Point::new(x0, y0),
        Point::new(x1, y1),
        stops,
    ))
}

pub(super) fn draw_solid_strokes(scene: &mut SceneBuilder) {
    let fill = rgba(28, 36, 52, 255);
    for (index, align, style, width) in [
        (0.0, StrokeAlign::Centered, StrokeStyle::Solid, 2.0),
        (72.0, StrokeAlign::Inside, StrokeStyle::Solid, 4.0),
        (144.0, StrokeAlign::Outside, StrokeStyle::Solid, 4.0),
        (216.0, StrokeAlign::Centered, StrokeStyle::Dashed(6.0), 2.0),
    ] {
        scene.draw_rect(
            rect(24.0 + index, 24.0, 64.0, 64.0),
            SceneRectStyle::new(fill, CornerRadii::uniform(12.0), 10_000)
                .with_stroke(stroke_with(style, align, width)),
        );
    }
}

pub(super) fn draw_gradient_strokes(scene: &mut SceneBuilder) {
    let gradient = LinearGradient::new(
        Point::new(24.0, 104.0),
        Point::new(88.0, 168.0),
        smallvec![
            GradientStop::new(0.0, rgba(24, 92, 180, 255)),
            GradientStop::new(1.0, rgba(224, 88, 144, 255)),
        ],
    );
    for (index, align) in [
        StrokeAlign::Centered,
        StrokeAlign::Inside,
        StrokeAlign::Outside,
        StrokeAlign::Centered,
    ]
    .into_iter()
    .enumerate()
    {
        let style = if index == 3 {
            StrokeStyle::Dashed(6.0)
        } else {
            StrokeStyle::Solid
        };
        scene.draw_gradient(
            rect(24.0 + index as f32 * 72.0, 104.0, 64.0, 64.0),
            SceneGradientStyle::new(gradient.clone(), CornerRadii::uniform(12.0), 10_000)
                .with_stroke(stroke_with(style, align, 3.0)),
        );
    }
}

pub(super) const fn stroke_with(style: StrokeStyle, align: StrokeAlign, width: f32) -> Stroke {
    Stroke {
        color: rgba(242, 242, 250, 255),
        width,
        style,
        align,
    }
}

pub(super) const fn solid_stroke(width: f32) -> Stroke {
    stroke_with(StrokeStyle::Solid, StrokeAlign::Centered, width)
}

pub(super) fn ambient_shadow(elevation: f32) -> Shadow {
    Shadow::new(
        [0.0, (elevation * 0.35).max(0.5)],
        (elevation * 1.0).max(0.75),
        elevation * 0.4,
        [0.0, 0.0, 0.0, 0.18],
    )
}

pub(super) fn key_shadow(elevation: f32) -> Shadow {
    Shadow::new(
        [0.0, (elevation * 0.8).max(1.0)],
        (elevation * 1.35).max(1.0),
        0.0,
        [0.0, 0.0, 0.0, 0.35],
    )
}

pub(super) const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

pub(super) const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
