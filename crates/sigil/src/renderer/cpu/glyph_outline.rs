use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{
    FillRule, Mask, Path, PathBuilder, Pixmap, Rect as TsRect, Transform as TsTransform,
};

use crate::hal::{Point, RenderedGlyph};
use crate::renderer::glyph_place::{coverage_curve, pen_shift, text_gamma};
use crate::renderer::oblique::Oblique;
use crate::renderer::scene_command::{GlyphRun, Transform};

use super::shapes::{self, Brush, decode, solid};

struct PathPen<'a> {
    builder: &'a mut PathBuilder,
    x: f32,
    y: f32,
}

impl OutlinePen for PathPen<'_> {
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
    paragraph.for_each_font(|font_id, bytes, font_index| {
        let Ok(font) = FontRef::from_index(bytes, font_index) else {
            return;
        };
        let outlines = font.outline_glyphs();
        for g in glyphs
            .iter()
            .filter(|g| g.font_id == font_id && g.font_index == font_index)
        {
            let Some(glyph) = outlines.get(GlyphId::new(g.glyph_id)) else {
                continue;
            };
            let pen = Point::new(origin.x + g.x, origin.y + g.y);
            let shift = pen_shift(transform, pen, scale);
            let mut builder = PathBuilder::new();
            let mut pen = PathPen {
                builder: &mut builder,
                x: pen.x + shift.x,
                y: pen.y + shift.y,
            };
            let coords = coords(g);
            let mut sheared = Oblique::new(&mut pen, g.skew);
            let settings = DrawSettings::unhinted(Size::new(g.size), LocationRef::new(&coords));
            let _ = glyph.draw(settings, &mut sheared);
            paths.extend(builder.finish());
        }
    });
    paths
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "coverage bytes and target sizes are bounded; float-to-byte rounding is the rasterizer contract"
)]
pub(super) fn fill_glyphs(
    target: &mut Pixmap,
    paths: &[Path],
    run: &GlyphRun,
    opacity: f32,
    brush: &Brush<'_>,
) {
    let paint = solid(run.color, opacity);
    let linear = [run.color.r, run.color.g, run.color.b].map(decode);
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

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "coverage bytes and target sizes are bounded; float-to-byte rounding is the rasterizer contract"
)]
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

fn coords(g: &RenderedGlyph) -> Vec<NormalizedCoord> {
    g.variation_coords
        .iter()
        .map(|&c| NormalizedCoord::from_bits(c))
        .collect()
}
