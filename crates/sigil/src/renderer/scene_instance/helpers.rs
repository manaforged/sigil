use crate::geometry::Rect;

use crate::renderer::scene_command::walk::IDENTITY;
use crate::renderer::scene_command::{Stroke, StrokeBand};
use crate::scene::corner_radii::CornerRadii;

use super::{BlendSpace, RectInstance, affine_rows};

#[derive(Clone, Copy)]
pub(super) struct RectPaint {
    pub(super) color: [f32; 4],
    pub(super) opacity: f32,
    pub(super) stroke: Option<Stroke>,
    pub(super) space: BlendSpace,
}

pub(super) fn rect_instance_parts(
    rect: Rect,
    corner_radii: CornerRadii,
    paint: RectPaint,
) -> RectInstance {
    let RectPaint {
        color,
        opacity,
        stroke,
        space,
    } = paint;
    let (stroke_params, stroke_meta) = stroke_fields(stroke, rect, space);
    let outset = stroke_outset(stroke, rect);
    let inflated = inflate(rect, outset);
    let [xform_x, xform_y] = affine_rows(IDENTITY);
    RectInstance {
        rect: rect_fields(inflated),
        color,
        corner_radii: corner_radii.normalized_for(rect.width, rect.height),
        params: [opacity.clamp(0.0, 1.0), outset, 0.0, 0.0],
        stroke_params,
        stroke_meta,
        xform_x,
        xform_y,
    }
}

pub(super) fn stroke_outset(stroke: Option<Stroke>, rect: Rect) -> f32 {
    StrokeBand::reach(stroke, rect)
}

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
