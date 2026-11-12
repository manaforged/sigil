use vello_cpu::kurbo::{Affine, BezPath, Cap, Join, Stroke};
use vello_cpu::peniko::{
    Extend, Fill, Gradient as Shape, InterpolationAlphaSpace, color::ColorSpaceTag,
};
use vello_cpu::{PaintType, color::AlphaColor, color::Srgb};

use crate::geometry::Point;
use crate::scene::command::{
    DrawPath, FillRule, Gradient, GradientKind, MITER_LIMIT, PathCmd, PathPaint, PathStroke,
    StrokeCap, StrokeJoin, Transform,
};
use crate::scene::geometry::{MIN_EXTENT, QUARTER_TURN_DEG};
use crate::style::Color;

pub(crate) trait PathSink {
    fn transform(&mut self, transform: Affine);
    fn paint(&mut self, paint: PaintType);
    fn paint_transform(&mut self, transform: Affine);
    fn fill(&mut self, path: &BezPath, rule: Fill);
    fn stroke(&mut self, path: &BezPath, stroke: Stroke);
}

macro_rules! impl_path_sink {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl PathSink for $ty {
                fn transform(&mut self, transform: Affine) {
                    self.set_transform(transform);
                }

                fn paint(&mut self, paint: PaintType) {
                    self.set_paint(paint);
                }

                fn paint_transform(&mut self, transform: Affine) {
                    self.set_paint_transform(transform);
                }

                fn fill(&mut self, path: &BezPath, rule: Fill) {
                    self.set_fill_rule(rule);
                    self.fill_path(path);
                }

                fn stroke(&mut self, path: &BezPath, stroke: Stroke) {
                    self.set_stroke(stroke);
                    self.stroke_path(path);
                }
            }
        )+
    };
}

impl_path_sink!(vello_cpu::RenderContext, vello_gpu::Scene);

pub(crate) fn record(sink: &mut impl PathSink, draw: &DrawPath, device: Transform, opacity: f32) {
    let path = outline(&draw.outline);
    sink.transform(affine(device) * affine(draw.transform));
    if let Some((rule, paint)) = &draw.fill {
        set_paint(sink, paint, opacity);
        sink.fill(&path, fill_rule(*rule));
    }
    if let Some(stroke) = &draw.stroke {
        set_paint(sink, &stroke.paint, opacity);
        sink.stroke(&path, kurbo_stroke(stroke));
    }
}

pub(crate) fn affine([[sx, kx, tx], [ky, sy, ty]]: Transform) -> Affine {
    Affine::new([sx, ky, kx, sy, tx, ty].map(f64::from))
}

fn outline(cmds: &[PathCmd]) -> BezPath {
    let mut path = BezPath::new();
    let point = |x: f32, y: f32| (f64::from(x), f64::from(y));
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => path.move_to(point(x, y)),
            PathCmd::LineTo(x, y) => path.line_to(point(x, y)),
            PathCmd::QuadTo(cx, cy, x, y) => path.quad_to(point(cx, cy), point(x, y)),
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                path.curve_to(point(c1x, c1y), point(c2x, c2y), point(x, y));
            }
            PathCmd::Close => path.close_path(),
        }
    }
    path
}

const fn fill_rule(rule: FillRule) -> Fill {
    match rule {
        FillRule::NonZero => Fill::NonZero,
        FillRule::EvenOdd => Fill::EvenOdd,
    }
}

fn kurbo_stroke(stroke: &PathStroke) -> Stroke {
    let cap = match stroke.cap {
        StrokeCap::Butt => Cap::Butt,
        StrokeCap::Round => Cap::Round,
        StrokeCap::Square => Cap::Square,
    };
    let join = match stroke.join {
        StrokeJoin::Miter => Join::Miter,
        StrokeJoin::Round => Join::Round,
        StrokeJoin::Bevel => Join::Bevel,
    };
    let line = Stroke::new(f64::from(stroke.width))
        .with_caps(cap)
        .with_join(join)
        .with_miter_limit(f64::from(MITER_LIMIT));
    if stroke.dash.iter().any(|length| *length > 0.0) {
        line.with_dashes(
            0.0,
            stroke.dash.iter().map(|length| f64::from(length.max(0.0))),
        )
    } else {
        line
    }
}

const fn color(color: Color, opacity: f32) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(color.r, color.g, color.b, color.a).multiply_alpha(opacity)
}

fn set_paint(sink: &mut impl PathSink, paint: &PathPaint, opacity: f32) {
    let (brush, placement) = match paint {
        PathPaint::Solid(solid) => (PaintType::Solid(color(*solid, opacity)), Affine::IDENTITY),
        PathPaint::Gradient(gradient) => gradient_brush(&gradient.normalized(), opacity),
    };
    sink.paint_transform(placement);
    sink.paint(brush);
}

fn gradient_brush(gradient: &Gradient, opacity: f32) -> (PaintType, Affine) {
    if let [only] = gradient.stops.as_slice() {
        return (
            PaintType::Solid(color(only.color, opacity)),
            Affine::IDENTITY,
        );
    }
    let ([first, last], stops) = gradient.spread();
    let extend = if gradient.repeat_span().is_some() {
        Extend::Repeat
    } else {
        Extend::Pad
    };
    let (shape, placement) = shape(gradient.kind, first, last);
    let stops: Vec<(f32, AlphaColor<Srgb>)> = stops
        .iter()
        .map(|stop| (stop.offset, color(stop.color, opacity)))
        .collect();
    let shape = shape
        .with_extend(extend)
        .with_interpolation_cs(ColorSpaceTag::Srgb)
        .with_interpolation_alpha_space(InterpolationAlphaSpace::Premultiplied)
        .with_stops(stops.as_slice());
    (PaintType::Gradient(shape), placement)
}

fn shape(kind: GradientKind, first: f32, last: f32) -> (Shape, Affine) {
    let point = |p: Point| (f64::from(p.x), f64::from(p.y));
    match kind {
        GradientKind::Linear { start, end } => {
            let at = |t: f32| {
                Point::new(
                    (end.x - start.x).mul_add(t, start.x),
                    (end.y - start.y).mul_add(t, start.y),
                )
            };
            (
                Shape::new_linear(point(at(first)), point(at(last))),
                Affine::IDENTITY,
            )
        }
        GradientKind::Radial { center, radius } => {
            let [rx, ry] = radius.map(|r| f64::from(r.max(MIN_EXTENT)));
            let place = Affine::translate(point(center)) * Affine::scale_non_uniform(rx, ry);
            (
                Shape::new_two_point_radial((0.0, 0.0), first, (0.0, 0.0), last),
                place,
            )
        }
        GradientKind::Conic { center, angle } => {
            let turn = f64::from((angle - QUARTER_TURN_DEG).to_radians());
            let place = Affine::translate(point(center)) * Affine::rotate(turn);
            let tau = std::f32::consts::TAU;
            (Shape::new_sweep((0.0, 0.0), first * tau, last * tau), place)
        }
    }
}
