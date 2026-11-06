use tiny_skia::{
    GradientStop as TsStop, LinearGradient as TsLinear, Point as TsPoint, RadialGradient, Shader,
    SpreadMode, SweepGradient, Transform as TsTransform,
};

use crate::geometry::Point;
use crate::renderer::scene_command::{Gradient, GradientKind, GradientStop};

use super::shapes::ts_color;

pub(super) fn gradient(gradient: &Gradient, opacity: f32) -> Option<Shader<'static>> {
    let gradient = gradient.normalized();
    if let [only] = gradient.stops.as_slice() {
        return Some(Shader::SolidColor(ts_color(only.color, opacity)));
    }
    let span = gradient.repeat_span();
    let [first, last] = span.unwrap_or([0.0, 1.0]);
    let mode = if span.is_some() {
        SpreadMode::Repeat
    } else {
        SpreadMode::Pad
    };
    let stops = rescaled(&gradient.stops, first, last, opacity);
    match gradient.kind {
        GradientKind::Linear { start, end } => {
            let at = |t: f32| TsPoint::from_xy(lerp(start.x, end.x, t), lerp(start.y, end.y, t));
            TsLinear::new(at(first), at(last), stops, mode, TsTransform::identity())
        }
        GradientKind::Radial { center, radius } => RadialGradient::new(
            TsPoint::zero(),
            first,
            TsPoint::zero(),
            last,
            stops,
            mode,
            around(center).pre_scale(radius[0].max(MIN_RADIUS), radius[1].max(MIN_RADIUS)),
        ),
        GradientKind::Conic { center, angle } => SweepGradient::new(
            TsPoint::zero(),
            first * TURN,
            last * TURN,
            stops,
            mode,
            around(center).pre_rotate(angle - QUARTER),
        ),
    }
}

const TURN: f32 = 360.0;
const QUARTER: f32 = 90.0;
const MIN_RADIUS: f32 = 1.0 / 4096.0;

fn around(center: Point) -> TsTransform {
    TsTransform::from_translate(center.x, center.y)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (b - a).mul_add(t, a)
}

fn rescaled(stops: &[GradientStop], first: f32, last: f32, opacity: f32) -> Vec<TsStop> {
    let width = (last - first).max(f32::EPSILON);
    stops
        .iter()
        .map(|stop| {
            let offset = ((stop.offset - first) / width).clamp(0.0, 1.0);
            TsStop::new(offset, ts_color(stop.color, opacity))
        })
        .collect()
}
