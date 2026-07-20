use sigil_hal::{Point, Rect};
use sigil_style::Rgba8;
use smallvec::smallvec;

use crate::scene_command::ScenePatternStyle;
use crate::{
    BUILTIN_SPRITE_ATLAS_ID, CornerRadii, Fill, GradientStop, LinearGradient, PatternKind,
    SceneBuilder, SceneGradientStyle, SceneRectStyle, Shadow, ShadowKind, SpriteSource, Stroke,
    StrokeAlign, StrokeStyle,
};

pub(super) const fn sprite_source(x: f32, y: f32, phase: f32) -> SpriteSource {
    SpriteSource {
        atlas: BUILTIN_SPRITE_ATLAS_ID,
        region: rect(x, y, 200.0, 200.0),
        phase,
    }
}

pub(super) fn draw_fill(scene: &mut SceneBuilder, rect: Rect, fill: Fill) {
    match fill {
        Fill::Solid(color) => scene.draw_rect(
            rect,
            SceneRectStyle::new(color, CornerRadii::uniform(16.0), 10_000),
        ),
        Fill::LinearGradient(gradient) => scene.draw_gradient(
            rect,
            SceneGradientStyle::new(gradient, CornerRadii::uniform(16.0), 10_000),
        ),
        Fill::Pattern(pattern) => scene.draw_pattern(
            rect,
            ScenePatternStyle::new(pattern, CornerRadii::uniform(16.0), 10_000),
        ),
    }
}

pub(super) fn slash_pattern() -> Fill {
    Fill::Pattern(slash_kind(rgba(255, 255, 255, 255), 40.0, 16.0))
}

pub(super) fn checker_pattern() -> Fill {
    Fill::Pattern(PatternKind::Checker {
        color_a: rgba(220, 60, 60, 255),
        color_b: rgba(245, 245, 250, 255),
        cell: 40.0,
    })
}

pub(super) const fn slash_kind(color: Rgba8, period: f32, line_width: f32) -> PatternKind {
    PatternKind::Slash {
        color,
        period,
        line_width,
    }
}

pub(super) fn subtle_vertical() -> Fill {
    linear_gradient(
        (120.0, 72.0),
        (120.0, 200.0),
        smallvec![
            GradientStop::new(0.0, rgba(34, 86, 160, 255)),
            GradientStop::new(1.0, rgba(19, 28, 48, 255)),
        ],
    )
}

pub(super) fn stripe_horizontal() -> Fill {
    linear_gradient(
        (216.0, 136.0),
        (360.0, 136.0),
        smallvec![
            GradientStop::new(0.0, rgba(99, 102, 241, 255)),
            GradientStop::new(1.0, rgba(14, 165, 233, 255)),
        ],
    )
}

pub(super) fn rainbow() -> Fill {
    linear_gradient(
        (384.0, 136.0),
        (528.0, 136.0),
        smallvec![
            GradientStop::new(0.0, rgba(239, 68, 68, 255)),
            GradientStop::new(0.34, rgba(245, 158, 11, 255)),
            GradientStop::new(0.67, rgba(34, 197, 94, 255)),
            GradientStop::new(1.0, rgba(59, 130, 246, 255)),
        ],
    )
}

pub(super) fn alpha_overlay() -> Fill {
    linear_gradient(
        (552.0, 72.0),
        (696.0, 200.0),
        smallvec![
            GradientStop::new(0.0, rgba(255, 255, 255, 24)),
            GradientStop::new(1.0, rgba(255, 255, 255, 224)),
        ],
    )
}

fn linear_gradient(
    start: (f32, f32),
    end: (f32, f32),
    stops: smallvec::SmallVec<[GradientStop; 4]>,
) -> Fill {
    Fill::LinearGradient(LinearGradient::new(
        Point::new(start.0, start.1),
        Point::new(end.0, end.1),
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
            SceneGradientStyle::new(stroke_gradient(), CornerRadii::uniform(12.0), 10_000)
                .with_stroke(stroke_with(style, align, 3.0)),
        );
    }
}

pub(super) fn stroke_gradient() -> LinearGradient {
    LinearGradient::new(
        Point::new(24.0, 104.0),
        Point::new(88.0, 168.0),
        smallvec![
            GradientStop::new(0.0, rgba(24, 92, 180, 255)),
            GradientStop::new(1.0, rgba(224, 88, 144, 255)),
        ],
    )
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

pub(super) fn corner_color_for(index: usize) -> Rgba8 {
    match index {
        0 => rgba(38, 92, 180, 255),
        1 => rgba(68, 156, 112, 255),
        2 => rgba(210, 98, 82, 255),
        3 => rgba(144, 86, 184, 255),
        4 => rgba(205, 136, 46, 255),
        _ => rgba(52, 132, 152, 255),
    }
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

pub(super) fn card_rect(index: usize) -> Rect {
    Rect::new(Point::new(80.0 + index as f32 * 180.0, 135.0), 130.0, 130.0)
}

pub(super) fn card_color(index: usize) -> Rgba8 {
    match index {
        0 => rgba(246, 248, 250, 255),
        1 => rgba(238, 245, 255, 255),
        2 => rgba(240, 250, 244, 255),
        3 => rgba(255, 244, 238, 255),
        4 => rgba(246, 242, 255, 255),
        _ => rgba(240, 248, 250, 255),
    }
}

pub(super) fn inner_press_shadow(index: usize) -> Shadow {
    Shadow::new([0.0, 2.0], 6.0, index as f32, [0.0, 0.0, 0.0, 0.45]).with_kind(ShadowKind::Inner)
}

pub(super) fn field_rect(index: usize) -> Rect {
    Rect::new(Point::new(70.0 + index as f32 * 220.0, 160.0), 200.0, 80.0)
}

pub(super) const fn field_color(_index: usize) -> Rgba8 {
    rgba(245, 247, 250, 255)
}

pub(super) const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

pub(super) const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
