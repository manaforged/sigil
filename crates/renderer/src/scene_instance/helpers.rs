//! Internal helpers for building [`super::RectInstance`] / [`super::ShadowInstance`]
//! payloads from public scene-command types.

use sigil_hal::Rect;

use crate::corner_radii::CornerRadii;
use crate::scene_command::{Stroke, StrokeAlign, StrokeStyle};

use super::{BlendSpace, RectInstance};

/// Fill + stroke visuals for one rect instance. `color` is the
/// blend-space-resolved `[r, g, b, a]` (see [`super::BlendSpace`]);
/// callers convert the source fill `Rgba8` so packing stays pure. The
/// stroke's `Rgba8` converts here through the same `space`.
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
    let (stroke_params, stroke_meta) = stroke_fields(stroke, space);
    // Inflate the draw quad outward by the stroke width so the
    // Outside/Centered halves of the stroke have actual fragments to
    // paint into. The vertex shader recovers the original surface
    // half-size by subtracting this outset, so the SDF math still
    // measures distance from the surface edge — not the expanded
    // quad edge.
    let outset = stroke_outset(stroke);
    let inflated = inflate(rect, outset);
    RectInstance {
        rect: rect_fields(inflated),
        color,
        corner_radii: corner_radii.nonnegative_array(),
        params: [opacity.clamp(0.0, 1.0), outset, 0.0, 0.0],
        stroke_params,
        stroke_meta,
    }
}

/// Per-side inflation in logical pixels that a draw quad needs so any
/// stroke alignment can render its outside half. Strokes with no
/// outside reach return 0; everything else returns the full stroke
/// width — slightly over-conservative for `Inside` but the extra
/// fragments evaluate to zero coverage and discard cheaply.
pub(super) fn stroke_outset(stroke: Option<Stroke>) -> f32 {
    stroke
        .map(|stroke| stroke.width.max(0.0))
        .unwrap_or_default()
}

pub(super) fn inflate(rect: Rect, outset: f32) -> Rect {
    if outset <= 0.0 {
        return rect;
    }
    Rect {
        origin: sigil_hal::Point::new(rect.origin.x - outset, rect.origin.y - outset),
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

pub(super) fn stroke_fields(stroke: Option<Stroke>, space: BlendSpace) -> ([f32; 4], [f32; 4]) {
    let Some(stroke) = stroke else {
        return ([0.0; 4], [0.0; 4]);
    };
    (
        space.rgba(stroke.color),
        [
            stroke.width.max(0.0),
            stroke_style_f32(stroke.style),
            stroke_align_f32(stroke.align),
            stroke_dash_gap(stroke.style),
        ],
    )
}

const fn stroke_style_f32(style: StrokeStyle) -> f32 {
    match style {
        StrokeStyle::Solid => 0.0,
        StrokeStyle::Dashed(_) => 1.0,
    }
}

const fn stroke_align_f32(align: StrokeAlign) -> f32 {
    match align {
        StrokeAlign::Centered => 0.0,
        StrokeAlign::Inside => 1.0,
        StrokeAlign::Outside => 2.0,
    }
}

const fn stroke_dash_gap(style: StrokeStyle) -> f32 {
    match style {
        StrokeStyle::Solid => 0.0,
        StrokeStyle::Dashed(gap) => gap,
    }
}
