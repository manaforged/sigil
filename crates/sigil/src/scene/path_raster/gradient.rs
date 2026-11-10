use tiny_skia::{
    Color as TsColor, ColorSpace, GradientStop as TsStop, LinearGradient as TsLinear,
    Point as TsPoint, RadialGradient, Shader, SpreadMode, SweepGradient, Transform as TsTransform,
};

use crate::animation::runtime::binding_animate_color::mix;
use crate::geometry::Point;
use crate::scene::command::{Gradient, GradientKind, GradientStop};
use crate::style::Rgba8;

const TURN: f32 = 360.0;
const QUARTER: f32 = 90.0;
const MIN_RADIUS: f32 = 1.0 / 4096.0;
const SUBSTEPS: u8 = 8;

/// The tiny-skia shader for `gradient` and the color space to blend it in.
pub(crate) fn shader(gradient: &Gradient, opacity: f32) -> Option<(Shader<'static>, ColorSpace)> {
    let gradient = gradient.normalized();
    let opaque = opacity >= 1.0 && gradient.stops.iter().all(|stop| stop.color.a == u8::MAX);
    let space = if opaque {
        ColorSpace::Linear
    } else {
        ColorSpace::Gamma2
    };
    if let [only] = gradient.stops.as_slice() {
        return Some((Shader::SolidColor(color(only.color, opacity)), space));
    }
    let ([first, last], stops) = gradient.spread();
    let mode = if gradient.repeat_span().is_some() {
        SpreadMode::Repeat
    } else {
        SpreadMode::Pad
    };
    let stops = if opaque {
        stops.to_vec()
    } else {
        dense(&stops)
    };
    let stops = stops
        .iter()
        .map(|stop| TsStop::new(stop.offset, color(stop.color, opacity)))
        .collect();
    let shader = match gradient.kind {
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
    }?;
    Some((shader, space))
}

fn dense(stops: &[GradientStop]) -> Vec<GradientStop> {
    let mut out: Vec<GradientStop> = stops.first().copied().into_iter().collect();
    for pair in stops.windows(2) {
        let &[a, b] = pair else {
            continue;
        };
        if b.offset > a.offset && a.color != b.color {
            out.extend((1..SUBSTEPS).map(|step| {
                let t = f32::from(step) / f32::from(SUBSTEPS);
                GradientStop::new(lerp(a.offset, b.offset, t), mix(a.color, b.color, t))
            }));
        }
        out.push(b);
    }
    out
}

fn color(color: Rgba8, opacity: f32) -> TsColor {
    let alpha = (f32::from(color.a) / 255.0 * opacity).clamp(0.0, 1.0);
    let channel = |value: u8| f32::from(value) / 255.0;
    TsColor::from_rgba(channel(color.r), channel(color.g), channel(color.b), alpha)
        .unwrap_or(TsColor::TRANSPARENT)
}

fn around(center: Point) -> TsTransform {
    TsTransform::from_translate(center.x, center.y)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (b - a).mul_add(t, a)
}
