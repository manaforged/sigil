#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use std::f32::consts::TAU;

use sigil_hal::{Point, Rect};
use sigil_style::Rgba8;
use tiny_skia::{
    Color as TsColor, FillRule, GradientStop as TsStop, LinearGradient as TsGradient, Mask, Paint,
    Path, PathBuilder, Pixmap, Point as TsPoint, Shader, SpreadMode, Stroke as TsStroke,
    StrokeDash, Transform as TsTransform,
};

use crate::corner_radii::CornerRadii;
use crate::scene_command::{
    DecorationKind, DrawDecoration, LinearGradient, Stroke, StrokeAlign, StrokeStyle, Transform,
};

const KAPPA: f32 = 0.552_284_8;
const WAVE_STEP: f32 = 1.0;

pub(super) struct Brush<'a> {
    pub(super) transform: TsTransform,
    pub(super) mask: Option<&'a Mask>,
    pub(super) opacity: f32,
}

pub(super) fn ts_transform(transform: Transform, scale: f32, origin: Point) -> TsTransform {
    let [[sx, kx, tx], [ky, sy, ty]] = transform;
    TsTransform::from_row(sx, ky, kx, sy, tx, ty)
        .post_scale(scale, scale)
        .post_translate(-origin.x, -origin.y)
}

pub(super) fn color(color: sigil_hal::Color) -> TsColor {
    TsColor::from_rgba(
        encode(color.r),
        encode(color.g),
        encode(color.b),
        color.a.clamp(0.0, 1.0),
    )
    .unwrap_or(TsColor::TRANSPARENT)
}

pub(super) fn encode(linear: f32) -> f32 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055_f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    }
}

pub(super) fn ts_color(color: Rgba8, opacity: f32) -> TsColor {
    let alpha = (f32::from(color.a) / 255.0 * opacity).clamp(0.0, 1.0);
    TsColor::from_rgba(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        alpha,
    )
    .unwrap_or(TsColor::TRANSPARENT)
}

pub(super) fn solid(color: Rgba8, opacity: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(ts_color(color, opacity));
    paint.anti_alias = true;
    paint
}

pub(super) fn opacity_of(opacity: u16, layer: f32) -> f32 {
    (f32::from(opacity) / 10_000.0 * layer).clamp(0.0, 1.0)
}

const ARC_DIRS: [[(f32, f32); 2]; 4] = [
    [(1.0, 0.0), (0.0, 1.0)],
    [(0.0, 1.0), (-1.0, 0.0)],
    [(-1.0, 0.0), (0.0, -1.0)],
    [(0.0, -1.0), (1.0, 0.0)],
];

pub(super) fn rounded_rect(rect: Rect, radii: CornerRadii) -> Option<Path> {
    let (left, top, width, height) = (rect.origin.x, rect.origin.y, rect.width, rect.height);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let limit = width.min(height) / 2.0;
    let [tl, tr, br, bl] = radii.0.map(|r| r.clamp(0.0, limit));
    if tl + tr + br + bl <= 0.0 {
        return PathBuilder::from_rect(tiny_skia::Rect::from_xywh(left, top, width, height)?)
            .into();
    }
    let (right, bottom) = (left + width, top + height);
    let mut b = PathBuilder::new();
    b.move_to(left + tl, top);
    b.line_to(right - tr, top);
    arc(&mut b, tr, (right, top + tr), 0);
    b.line_to(right, bottom - br);
    arc(&mut b, br, (right - br, bottom), 1);
    b.line_to(left + bl, bottom);
    arc(&mut b, bl, (left, bottom - bl), 2);
    b.line_to(left, top + tl);
    arc(&mut b, tl, (left + tl, top), 3);
    b.close();
    b.finish()
}

fn arc(b: &mut PathBuilder, r: f32, to: (f32, f32), corner: usize) {
    if r <= 0.0 {
        return;
    }
    let Some([d0, d1]) = ARC_DIRS.get(corner).copied() else {
        return;
    };
    let from = (
        (d0.0 + d1.0).mul_add(-r, to.0),
        (d0.1 + d1.1).mul_add(-r, to.1),
    );
    let k = KAPPA * r;
    b.cubic_to(
        d0.0.mul_add(k, from.0),
        d0.1.mul_add(k, from.1),
        d1.0.mul_add(-k, to.0),
        d1.1.mul_add(-k, to.1),
        to.0,
        to.1,
    );
}

pub(super) fn fill(target: &mut Pixmap, path: &Path, paint: &Paint<'_>, brush: &Brush<'_>) {
    target.fill_path(path, paint, FillRule::Winding, brush.transform, brush.mask);
}

pub(super) fn stroke(
    target: &mut Pixmap,
    rect: Rect,
    radii: CornerRadii,
    stroke: Stroke,
    brush: &Brush<'_>,
) {
    let half = stroke.width / 2.0;
    let (rect, radii) = match stroke.align {
        StrokeAlign::Centered => (rect, radii),
        StrokeAlign::Inside => (
            inset(rect, half),
            CornerRadii(radii.0.map(|r| (r - half).max(0.0))),
        ),
        StrokeAlign::Outside => (inset(rect, -half), CornerRadii(radii.0.map(|r| r + half))),
    };
    let Some(path) = rounded_rect(rect, radii) else {
        return;
    };
    let dash = match stroke.style {
        StrokeStyle::Dashed(gap) if gap > 0.0 => {
            StrokeDash::new(vec![(stroke.width * 3.0).max(1.0), gap], 0.0)
        }
        StrokeStyle::Solid | StrokeStyle::Dashed(_) => None,
    };
    let ts = TsStroke {
        width: stroke.width,
        dash,
        ..TsStroke::default()
    };
    target.stroke_path(
        &path,
        &solid(stroke.color, brush.opacity),
        &ts,
        brush.transform,
        brush.mask,
    );
}

fn inset(rect: Rect, by: f32) -> Rect {
    Rect::new(
        Point::new(rect.origin.x + by, rect.origin.y + by),
        by.mul_add(-2.0, rect.width).max(0.0),
        by.mul_add(-2.0, rect.height).max(0.0),
    )
}

pub(super) fn gradient(gradient: &LinearGradient, opacity: f32) -> Option<Shader<'static>> {
    let stops = gradient
        .stops
        .iter()
        .map(|stop| TsStop::new(stop.offset.clamp(0.0, 1.0), ts_color(stop.color, opacity)))
        .collect();
    TsGradient::new(
        TsPoint::from_xy(gradient.start.x, gradient.start.y),
        TsPoint::from_xy(gradient.end.x, gradient.end.y),
        stops,
        SpreadMode::Pad,
        TsTransform::identity(),
    )
}

pub(super) fn decoration(target: &mut Pixmap, draw: &DrawDecoration, brush: &Brush<'_>) {
    let band = draw.band;
    let paint = solid(draw.color, opacity_of(draw.opacity, brush.opacity));
    let center = band.origin.y + band.height / 2.0;
    match draw.kind {
        DecorationKind::Underline | DecorationKind::Strikethrough => {
            let Some(rect) = tiny_skia::Rect::from_xywh(
                band.origin.x,
                center - draw.line_width / 2.0,
                band.width,
                draw.line_width,
            ) else {
                return;
            };
            target.fill_rect(rect, &paint, brush.transform, brush.mask);
        }
        DecorationKind::WavyUnderline => {
            let period = draw.period.max(1.0);
            let amplitude = draw.line_width * 0.75;
            let mut b = PathBuilder::new();
            let steps = (band.width / WAVE_STEP).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let x = (step as f32 * WAVE_STEP).min(band.width);
                let y = amplitude.mul_add((x / period * TAU).sin(), center);
                if step == 0 {
                    b.move_to(band.origin.x, y);
                } else {
                    b.line_to(band.origin.x + x, y);
                }
            }
            let Some(path) = b.finish() else {
                return;
            };
            let ts = TsStroke {
                width: draw.line_width * 0.8,
                ..TsStroke::default()
            };
            target.stroke_path(&path, &paint, &ts, brush.transform, brush.mask);
        }
    }
}
