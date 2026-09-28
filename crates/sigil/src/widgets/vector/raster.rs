use tiny_skia::{
    Color as TsColor, GradientStop, LineCap, LineJoin, LinearGradient, Paint,
    PathBuilder as TsPathBuilder, Pixmap, Point as TsPoint, RadialGradient, Shader, SpreadMode,
    Stroke, Transform,
};

use super::{PathCmd, PathFill};
use crate::style::Rgba8;

pub(super) fn build_ts_path(cmds: &[PathCmd]) -> Option<tiny_skia::Path> {
    let mut builder = TsPathBuilder::new();
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => builder.move_to(x, y),
            PathCmd::LineTo(x, y) => builder.line_to(x, y),
            PathCmd::QuadTo(cx, cy, x, y) => builder.quad_to(cx, cy, x, y),
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                builder.cubic_to(c1x, c1y, c2x, c2y, x, y);
            }
            PathCmd::Close => builder.close(),
        }
    }
    builder.finish()
}

pub(super) fn straight_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixmap.pixels().len().saturating_mul(4));
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        out.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    out
}

pub(super) fn stroke_path(
    pixmap: &mut Pixmap,
    path: &tiny_skia::Path,
    (color, width): (Rgba8, f32),
    transform: Transform,
    fixed_scale: Option<f32>,
) {
    let mut paint = Paint::default();
    paint.set_color(to_ts_color(color));
    paint.anti_alias = true;
    let Some(scale) = fixed_scale else {
        let line = Stroke {
            width,
            ..Stroke::default()
        };
        pixmap.stroke_path(path, &paint, &line, transform, None);
        return;
    };
    let Some(mapped) = path.clone().transform(transform) else {
        return;
    };
    let line = Stroke {
        width: width * scale,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&mapped, &paint, &line, Transform::identity(), None);
}

pub(super) fn to_ts_color(color: Rgba8) -> TsColor {
    TsColor::from_rgba8(color.r, color.g, color.b, color.a)
}

pub(super) fn fill_shader(fill: &PathFill) -> Shader<'static> {
    let transparent = Shader::SolidColor(TsColor::TRANSPARENT);
    match fill {
        PathFill::Solid(color) => Shader::SolidColor(to_ts_color(*color)),
        PathFill::Theme(_) => transparent,
        PathFill::Linear { start, end, stops } => LinearGradient::new(
            point(*start),
            point(*end),
            ts_stops(stops),
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap_or(transparent),
        PathFill::Radial {
            center,
            radius,
            stops,
        } => {
            let focus = point(*center);
            RadialGradient::new(
                focus,
                0.0,
                focus,
                *radius,
                ts_stops(stops),
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap_or(transparent)
        }
    }
}

fn ts_stops(stops: &[(f32, Rgba8)]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|&(position, color)| GradientStop::new(position, to_ts_color(color)))
        .collect()
}

fn point((x, y): (f32, f32)) -> TsPoint {
    TsPoint::from_xy(x, y)
}
