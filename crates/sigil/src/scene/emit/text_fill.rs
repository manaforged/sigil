use std::sync::Arc;

use skrifa::outline::OutlinePen;

use crate::scene::command::fills::{FillRule, color_at};
use crate::scene::command::{DrawPath, GlyphRun, Gradient, PathCommand, PathPaint, outline_key};
use crate::scene::geometry::IDENTITY;
use crate::text::outline::{Oblique, for_each_outline};

const TEXT_FILL_SEED: u64 = 0x7478_7466;

struct Pen<'a> {
    out: &'a mut Vec<PathCommand>,
    x: f32,
    y: f32,
}

impl Pen<'_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x, self.y - y)
    }
}

impl OutlinePen for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.out.push(PathCommand::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.out.push(PathCommand::LineTo(x, y));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let ((cx, cy), (x, y)) = (self.at(cx, cy), self.at(x, y));
        self.out.push(PathCommand::QuadTo(cx, cy, x, y));
    }

    fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
        let ((ax, ay), (bx, by), (x, y)) = (self.at(ax, ay), self.at(bx, by), self.at(x, y));
        self.out.push(PathCommand::CubicTo(ax, ay, bx, by, x, y));
    }

    fn close(&mut self) {
        self.out.push(PathCommand::Close);
    }
}

/// The glyph outlines of `run` filled with `gradient`, or `None` when the run has no outlines.
pub(super) fn glyph_fill(run: &GlyphRun, gradient: &Gradient) -> Option<DrawPath> {
    let outline = outlines(run);
    if outline.is_empty() {
        return None;
    }
    let gradient = gradient.normalized();
    let key = outline_key(gradient.key(TEXT_FILL_SEED), &outline);
    let paint = PathPaint::Gradient(gradient);
    let mut draw =
        DrawPath::keyed(Arc::from(outline), IDENTITY, key).filled(FillRule::NonZero, paint);
    draw.node = run.node;
    draw.opacity = run.opacity_bp();
    Some(draw)
}

/// The color a host that cannot fill glyphs with a gradient draws the text in.
pub(super) fn flat_color(gradient: &Gradient) -> crate::style::Color {
    color_at(&gradient.normalized().stops, 0.5)
}

fn outlines(run: &GlyphRun) -> Vec<PathCommand> {
    let mut out = Vec::new();
    let Some(paragraph) = run.paragraph() else {
        return out;
    };
    let glyphs = run.positioned_glyphs();
    let origin = run.rect.origin;
    for_each_outline(paragraph, glyphs, |g, glyph, settings| {
        let start = out.len();
        let mut pen = Pen {
            out: &mut out,
            x: origin.x + g.x,
            y: origin.y + g.y,
        };
        if let Err(error) = glyph.draw(settings, &mut Oblique::new(&mut pen, g.skew)) {
            tracing::warn!(glyph = g.glyph_id, %error, "gradient text skips a glyph it cannot outline");
            out.truncate(start);
        }
    });
    out
}
