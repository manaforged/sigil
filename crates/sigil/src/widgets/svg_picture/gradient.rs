use resvg::usvg::{self, BaseGradient, SpreadMethod, Transform};
use smallvec::SmallVec;

use crate::geometry::Point;
use crate::scene::command::{Gradient, GradientKind, GradientStop, PathPaint};
use crate::style::Rgba8;

use super::fallback::Fallback;
use super::paint::{Fill, alpha};

const EPSILON: f32 = 1e-5;
const SPLIT: usize = 8;

type Stop = (f32, [f32; 4]);

struct Stops {
    list: SmallVec<[GradientStop; 4]>,
    reach: f32,
    repeat: bool,
}

pub(super) fn linear(gradient: &usvg::LinearGradient, opacity: f32) -> Fill {
    let Stops {
        list: stops,
        reach,
        repeat,
    } = stops(gradient, opacity);
    let t = gradient.transform();
    let dx = (gradient.x2() - gradient.x1()) * reach;
    let dy = (gradient.y2() - gradient.y1()) * reach;
    let start = map(t, gradient.x1(), gradient.y1());
    let end = axis_end(t, start, dx, dy);
    let kind = GradientKind::Linear { start, end };
    Fill::Plain(PathPaint::Gradient(Gradient {
        kind,
        stops,
        repeat,
    }))
}

fn axis_end(t: Transform, start: Point, dx: f32, dy: f32) -> Point {
    let length = dx.mul_add(dx, dy * dy);
    let Some(inverse) = t.invert().filter(|_| length > EPSILON) else {
        return start;
    };
    let ax = inverse.sx.mul_add(dx, inverse.ky * dy) / length;
    let ay = inverse.kx.mul_add(dx, inverse.sy * dy) / length;
    let reach = ax.mul_add(ax, ay * ay);
    if reach <= f32::MIN_POSITIVE {
        return start;
    }
    Point::new(start.x + ax / reach, start.y + ay / reach)
}

pub(super) fn radial(gradient: &usvg::RadialGradient, opacity: f32) -> Result<Fill, Fallback> {
    let focal = (gradient.fx() - gradient.cx()).abs() > EPSILON
        || (gradient.fy() - gradient.cy()).abs() > EPSILON
        || gradient.fr().get() > EPSILON;
    if focal {
        return Err(Fallback::FocalGradient);
    }
    let Stops {
        list: stops,
        reach,
        repeat,
    } = stops(gradient, opacity);
    let radius = gradient.r().get() * reach;
    let t = gradient.transform();
    let paint = |center: Point, radii: [f32; 2]| {
        let kind = GradientKind::Radial {
            center,
            radius: radii,
        };
        PathPaint::Gradient(Gradient {
            kind,
            stops,
            repeat,
        })
    };
    if t.kx.abs() < EPSILON && t.ky.abs() < EPSILON {
        let center = map(t, gradient.cx(), gradient.cy());
        return Ok(Fill::Plain(paint(
            center,
            [radius * t.sx.abs(), radius * t.sy.abs()],
        )));
    }
    let center = Point::new(gradient.cx(), gradient.cy());
    match t.invert() {
        Some(_) => Ok(Fill::Space(paint(center, [radius; 2]), t)),
        None => Ok(Fill::Plain(paint(center, [0.0; 2]))),
    }
}

fn map(t: Transform, x: f32, y: f32) -> Point {
    Point::new(
        t.sx.mul_add(x, t.kx.mul_add(y, t.tx)),
        t.ky.mul_add(x, t.sy.mul_add(y, t.ty)),
    )
}

fn stops(base: &BaseGradient, opacity: f32) -> Stops {
    let raw: Vec<Stop> = base
        .stops()
        .iter()
        .map(|stop| {
            let color = stop.color();
            let rgb =
                [color.red, color.green, color.blue].map(|channel| f32::from(channel) / 255.0);
            let [r, g, b] = rgb;
            (
                stop.offset().get(),
                [r, g, b, stop.opacity().get() * opacity],
            )
        })
        .collect();
    let (spread, reach, repeat) = match base.spread_method() {
        SpreadMethod::Pad => (raw, 1.0, false),
        SpreadMethod::Repeat => (padded(raw), 1.0, true),
        SpreadMethod::Reflect => (mirrored(&padded(raw)), 2.0, true),
    };
    let stops = unpremultiplied(&spread)
        .into_iter()
        .map(|(offset, color)| GradientStop::new(offset, bytes(color)))
        .collect();
    Stops {
        list: stops,
        reach,
        repeat,
    }
}

fn padded(mut stops: Vec<Stop>) -> Vec<Stop> {
    if let Some(&(offset, color)) = stops.first()
        && offset > 0.0
    {
        stops.insert(0, (0.0, color));
    }
    if let Some(&(offset, color)) = stops.last()
        && offset < 1.0
    {
        stops.push((1.0, color));
    }
    stops
}

fn mirrored(stops: &[Stop]) -> Vec<Stop> {
    let forward = stops.iter().map(|&(offset, color)| (offset * 0.5, color));
    let back = stops
        .iter()
        .rev()
        .map(|&(offset, color)| (offset.mul_add(-0.5, 1.0), color));
    forward.chain(back).collect()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the split count is a small constant"
)]
fn unpremultiplied(stops: &[Stop]) -> Vec<Stop> {
    let mut out = Vec::with_capacity(stops.len() * SPLIT);
    for pair in stops.windows(2) {
        let &[(from, a), (to, b)] = pair else {
            continue;
        };
        out.push((from, a));
        let [ar, ag, ab, aa] = a;
        let [br, bg, bb, ba] = b;
        let hue = [ar - br, ag - bg, ab - bb]
            .iter()
            .any(|gap| gap.abs() > EPSILON);
        let differs = (aa - ba).abs() > 0.5 / 255.0 && hue;
        if !differs || to - from <= EPSILON {
            continue;
        }
        for step in 1..SPLIT {
            let t = step as f32 / SPLIT as f32;
            let mut color = a;
            for (channel, (start, end)) in color.iter_mut().zip(a.into_iter().zip(b)) {
                *channel = (end - start).mul_add(t, start);
            }
            out.push(((to - from).mul_add(t, from), color));
        }
    }
    out.extend(stops.last().copied());
    out
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each channel is clamped to 0..=1 before the cast"
)]
fn bytes([r, g, b, a]: [f32; 4]) -> Rgba8 {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba8 {
        r: byte(r),
        g: byte(g),
        b: byte(b),
        a: alpha(a),
    }
}
