#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "coverage bytes and target sizes are bounded; float-to-byte rounding is the rasterizer contract"
)]

use skrifa::outline::OutlinePen;
use tiny_skia::{
    FillRule, Mask, Path, PathBuilder, Pixmap, Rect as TsRect, Transform as TsTransform,
};

use crate::geometry::Point;
use crate::renderer::glyph_place::{coverage_curve, pen_shift, text_gamma};
use crate::renderer::scene_command::{GlyphRun, Transform};
use crate::text::outline::{Oblique, for_each_outline};

use super::shapes::{self, Brush, decode, solid};

struct GlyphPen<'a> {
    builder: &'a mut PathBuilder,
    x: f32,
    y: f32,
}

impl OutlinePen for GlyphPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(self.x + x, self.y - y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(self.x + x, self.y - y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.builder
            .quad_to(self.x + cx, self.y - cy, self.x + x, self.y - y);
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            self.x + c0x,
            self.y - c0y,
            self.x + c1x,
            self.y - c1y,
            self.x + x,
            self.y - y,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

pub(super) fn run_paths(run: &GlyphRun, transform: Transform, scale: f32) -> Vec<Path> {
    let Some(paragraph) = run.paragraph() else {
        return Vec::new();
    };
    let glyphs = run.positioned_glyphs();
    let origin = run.rect.origin;
    let mut paths = Vec::new();
    for_each_outline(paragraph, glyphs, |g, glyph, settings| {
        let pen = Point::new(origin.x + g.x, origin.y + g.y);
        let shift = pen_shift(transform, pen, scale);
        let mut builder = PathBuilder::new();
        let mut pen = GlyphPen {
            builder: &mut builder,
            x: pen.x + shift.x,
            y: pen.y + shift.y,
        };
        if glyph
            .draw(settings, &mut Oblique::new(&mut pen, g.skew))
            .is_ok()
        {
            paths.extend(builder.finish());
        }
    });
    paths
}

pub(super) fn fill_glyphs(
    target: &mut Pixmap,
    paths: &[Path],
    run: &GlyphRun,
    opacity: f32,
    brush: &Brush<'_>,
) {
    let color = run.color();
    let paint = solid(color, opacity);
    let linear = [color.r, color.g, color.b].map(decode);
    let gamma = text_gamma([linear[0], linear[1], linear[2], 1.0], true);
    for path in paths {
        if (gamma - 1.0).abs() < f32::EPSILON {
            shapes::fill(target, path, &paint, brush);
            continue;
        }
        let Some(mask) = curved_mask(target, path, gamma, brush) else {
            continue;
        };
        let Some(rect) = TsRect::from_xywh(0.0, 0.0, target.width() as f32, target.height() as f32)
        else {
            continue;
        };
        target.fill_rect(rect, &paint, TsTransform::identity(), Some(&mask));
    }
}

fn curved_mask(target: &Pixmap, path: &Path, gamma: f32, brush: &Brush<'_>) -> Option<Mask> {
    let mut mask = Mask::new(target.width(), target.height())?;
    mask.fill_path(path, FillRule::Winding, true, brush.transform);
    let table: [u8; 256] =
        core::array::from_fn(|i| (coverage_curve(i as f32 / 255.0, gamma) * 255.0).round() as u8);
    let clip = brush.coverage().map(Mask::data);
    for (index, value) in mask.data_mut().iter_mut().enumerate() {
        let curved = table.get(usize::from(*value)).copied().unwrap_or(*value);
        let limit = clip
            .and_then(|data| data.get(index))
            .copied()
            .unwrap_or(255);
        *value = ((u16::from(curved) * u16::from(limit) + 127) / 255) as u8;
    }
    Some(mask)
}
