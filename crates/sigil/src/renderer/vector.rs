use vello_cpu::kurbo::{Affine, BezPath, Cap, Join, Stroke};
use vello_cpu::peniko::{Brush, Fill, Gradient};
use vello_cpu::{PaintType, color::AlphaColor, color::Srgb};

use crate::scene::command::{
    DrawPath, FillRule, MITER_LIMIT, PathCmd, PathPaint, PathStroke, StrokeCap, StrokeJoin,
    Transform,
};
use crate::style::Rgba8;

pub(crate) trait PathSink {
    fn transform(&mut self, transform: Affine);
    fn paint(&mut self, paint: PaintType);
    fn fill(&mut self, path: &BezPath, rule: Fill);
    fn stroke(&mut self, path: &BezPath, stroke: Stroke);
}

impl PathSink for vello_cpu::RenderContext {
    fn transform(&mut self, transform: Affine) {
        self.set_transform(transform);
    }

    fn paint(&mut self, paint: PaintType) {
        self.set_paint(paint);
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

impl PathSink for vello_gpu::Scene {
    fn transform(&mut self, transform: Affine) {
        self.set_transform(transform);
    }

    fn paint(&mut self, paint: PaintType) {
        self.set_paint(paint);
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

pub(crate) fn record(sink: &mut impl PathSink, draw: &DrawPath, device: Transform, opacity: f32) {
    let path = outline(&draw.outline);
    sink.transform(affine(device) * affine(draw.transform));
    if let Some((rule, paint)) = &draw.fill {
        sink.paint(brush(paint, opacity));
        sink.fill(&path, fill_rule(*rule));
    }
    if let Some(stroke) = &draw.stroke {
        sink.paint(brush(&stroke.paint, opacity));
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
        line.with_dashes(0.0, stroke.dash.iter().map(|length| f64::from(length.max(0.0))))
    } else {
        line
    }
}

fn color(color: Rgba8, opacity: f32) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(color.r, color.g, color.b, color.a).multiply_alpha(opacity)
}

fn brush(paint: &PathPaint, opacity: f32) -> PaintType {
    let stops = |stops: &[crate::scene::command::GradientStop]| -> Vec<(f32, AlphaColor<Srgb>)> {
        stops
            .iter()
            .map(|stop| (stop.offset, color(stop.color, opacity)))
            .collect()
    };
    let point = |p: crate::geometry::Point| (f64::from(p.x), f64::from(p.y));
    match paint {
        PathPaint::Solid(solid) => Brush::Solid(color(*solid, opacity)),
        PathPaint::Linear(linear) => Brush::Gradient(
            Gradient::new_linear(point(linear.start), point(linear.end))
                .with_stops(stops(&linear.stops).as_slice()),
        ),
        PathPaint::Radial {
            center,
            radius,
            stops: list,
        } => Brush::Gradient(
            Gradient::new_radial(point(*center), *radius).with_stops(stops(list).as_slice()),
        ),
    }
}
