use tiny_skia::{
    Color as TsColor, ColorSpace, FillRule as TsFillRule, LineCap, LineJoin, Paint, PathBuilder,
    Pixmap, Shader, Stroke, StrokeDash, Transform as TsTransform,
};

use crate::scene::command::{
    DrawPath, FillRule, MITER_LIMIT, PathCmd, PathPaint, PathStroke, StrokeCap, StrokeJoin,
    Transform,
};
use crate::scene::geometry::compose;
use crate::style::Color;

pub(super) fn raster(
    draw: &DrawPath,
    device: Transform,
    (width, height): (u16, u16),
    opacity: f32,
) -> Option<Vec<u8>> {
    let path = outline(&draw.outline)?;
    let mut pixmap = Pixmap::new(u32::from(width), u32::from(height))?;
    let [[sx, kx, tx], [ky, sy, ty]] = compose(device, draw.transform);
    let transform = TsTransform::from_row(sx, ky, kx, sy, tx, ty);
    if let Some((rule, paint)) = &draw.fill {
        let rule = match rule {
            FillRule::NonZero => TsFillRule::Winding,
            FillRule::EvenOdd => TsFillRule::EvenOdd,
        };
        pixmap.fill_path(&path, &skia_paint(paint, opacity), rule, transform, None);
    }
    if let Some(stroke) = &draw.stroke {
        let paint = skia_paint(&stroke.paint, opacity);
        pixmap.stroke_path(&path, &paint, &skia_stroke(stroke), transform, None);
    }
    Some(pixmap.take())
}

fn outline(cmds: &[PathCmd]) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
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

fn skia_stroke(stroke: &PathStroke) -> Stroke {
    Stroke {
        width: stroke.width,
        miter_limit: MITER_LIMIT,
        line_cap: match stroke.cap {
            StrokeCap::Butt => LineCap::Butt,
            StrokeCap::Round => LineCap::Round,
            StrokeCap::Square => LineCap::Square,
        },
        line_join: match stroke.join {
            StrokeJoin::Miter => LineJoin::Miter,
            StrokeJoin::Round => LineJoin::Round,
            StrokeJoin::Bevel => LineJoin::Bevel,
        },
        dash: StrokeDash::new(stroke.dash.to_vec(), 0.0),
    }
}

fn color(color: Color, opacity: f32) -> TsColor {
    let mut out = TsColor::from_rgba8(color.r, color.g, color.b, color.a);
    out.apply_opacity(opacity);
    out
}

fn skia_paint(paint: &PathPaint, opacity: f32) -> Paint<'static> {
    let (shader, colorspace) = match paint {
        PathPaint::Solid(solid) => (
            Shader::SolidColor(color(*solid, opacity)),
            ColorSpace::Linear,
        ),
        PathPaint::Gradient(gradient) => super::gradient::shader(gradient, opacity)
            .unwrap_or((Shader::SolidColor(TsColor::TRANSPARENT), ColorSpace::Linear)),
    };
    Paint {
        shader,
        anti_alias: true,
        colorspace,
        ..Paint::default()
    }
}
