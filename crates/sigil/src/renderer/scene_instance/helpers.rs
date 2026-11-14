use crate::geometry::Rect;

use crate::renderer::scene_command::{Stroke, StrokeBand};

use super::BlendSpace;

pub(super) fn inflate(rect: Rect, outset: f32) -> Rect {
    if outset <= 0.0 {
        return rect;
    }
    Rect {
        origin: crate::geometry::Point::new(rect.origin.x - outset, rect.origin.y - outset),
        width: outset.mul_add(2.0, rect.width),
        height: outset.mul_add(2.0, rect.height),
    }
}

pub(super) const fn rect_fields(rect: Rect) -> [f32; 4] {
    let Rect {
        origin,
        width,
        height,
    } = rect;
    [origin.x, origin.y, width, height]
}

pub(super) fn shadow_color(color: [f32; 4], opacity_scale: f32) -> [f32; 4] {
    let [r, g, b, a] = color;
    [
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        (a * opacity_scale).clamp(0.0, 1.0),
    ]
}

pub(super) fn stroke_fields(
    stroke: Option<Stroke>,
    rect: Rect,
    space: BlendSpace,
) -> ([f32; 4], [f32; 4]) {
    let Some((stroke, band)) = stroke.and_then(|s| StrokeBand::of(s, rect).map(|b| (s, b))) else {
        return ([0.0; 4], [0.0; 4]);
    };
    let (length, gap) = band.dash.map_or((0.0, 0.0), |dash| (dash.length, dash.gap));
    (
        space.rgba(stroke.color),
        [band.inside, band.outside, length, gap],
    )
}
