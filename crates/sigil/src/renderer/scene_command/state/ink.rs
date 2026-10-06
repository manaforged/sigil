use crate::geometry::{Point, Rect};
use crate::renderer::glyph_place::run_ink;
use crate::renderer::scene_command::walk::transform_rect;
use crate::renderer::scene_command::{LayerEffect, SceneCommand, Stroke, Transform};
use crate::renderer::shadow_paint::Shadow;

pub(crate) fn invert(transform: Transform) -> Option<Transform> {
    let [[a, b, tx], [c, d, ty]] = transform;
    let det = a.mul_add(d, -(b * c));
    if !det.is_normal() {
        return None;
    }
    let inverse = [
        [d / det, -b / det, b.mul_add(ty, -(d * tx)) / det],
        [-c / det, a / det, c.mul_add(tx, -(a * ty)) / det],
    ];
    inverse
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(inverse)
}

pub(crate) fn shadow_reach(shadow: &Shadow) -> f32 {
    shadow.blur.mul_add(
        3.0,
        shadow.spread.abs() + shadow.offset[0].abs().max(shadow.offset[1].abs()),
    )
}

pub(crate) fn ink_rect(command: &SceneCommand, scale: f32) -> Option<Rect> {
    let stroked =
        |rect: Rect, stroke: Option<Stroke>| rect.expand(stroke.map_or(0.0, |s| s.width.max(0.0)));
    Some(match command {
        SceneCommand::Rect(draw) => {
            let ink = stroked(draw.rect, draw.stroke);
            draw.border.map_or(ink, |border| ink.union(border.rect))
        }
        SceneCommand::Gradient(draw) => {
            let ink = stroked(draw.rect, draw.stroke);
            draw.border.map_or(ink, |border| ink.union(border.rect))
        }
        SceneCommand::Pattern(draw) => stroked(draw.rect, draw.stroke),
        SceneCommand::DrawShadow { rect, shadow, .. } => {
            Rect::new(Point::new(rect[0], rect[1]), rect[2], rect[3]).expand(shadow_reach(shadow))
        }
        SceneCommand::GlyphRun(draw) => run_ink(&draw.run)?.expand(1.0 / scale),
        SceneCommand::Sprite(draw) => draw.destination,
        SceneCommand::Image(draw) => draw.destination,
        SceneCommand::Decoration(draw) => draw.band.expand(draw.line_width * 2.0),
        SceneCommand::PushLayer(scope) => match scope.effect {
            LayerEffect::Transform(transform) => transform_rect(transform, scope.clip),
            LayerEffect::Identity | LayerEffect::LayerBlur { .. } => scope.clip,
        },
        SceneCommand::PushClip(_)
        | SceneCommand::PopClip
        | SceneCommand::PushTransform(_)
        | SceneCommand::PopTransform
        | SceneCommand::SaveLayer { .. }
        | SceneCommand::RestoreLayer
        | SceneCommand::PopLayer => return None,
    })
}

pub(crate) fn snap_out(rect: Rect, grid: Point, scale: f32) -> Rect {
    let snap = |value: f32, origin: f32, round: fn(f32) -> f32| {
        round((value - origin) * scale) / scale + origin
    };
    let x0 = snap(rect.origin.x, grid.x, f32::floor);
    let y0 = snap(rect.origin.y, grid.y, f32::floor);
    let x1 = snap(rect.origin.x + rect.width, grid.x, f32::ceil);
    let y1 = snap(rect.origin.y + rect.height, grid.y, f32::ceil);
    Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0)
}
