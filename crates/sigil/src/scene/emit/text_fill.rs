use std::sync::Arc;

use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::scene::command::fills::{FillRule, color_at};
use crate::scene::command::{
    DrawPath, GlyphRun, Gradient, GradientKind, LinearGradient, PathCmd, PathPaint, Transform,
    outline_key,
};

struct Pen<'a> {
    out: &'a mut Vec<PathCmd>,
    x: f32,
    y: f32,
    skew: f32,
    squash: f32,
    pivot: f32,
}

impl Pen<'_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        let down = self.y - y;
        (self.skew.mul_add(y, self.x + x), (down - self.pivot).mul_add(self.squash, self.pivot))
    }
}

impl OutlinePen for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.out.push(PathCmd::MoveTo(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        self.out.push(PathCmd::LineTo(x, y));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let ((cx, cy), (x, y)) = (self.at(cx, cy), self.at(x, y));
        self.out.push(PathCmd::QuadTo(cx, cy, x, y));
    }

    fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
        let ((ax, ay), (bx, by), (x, y)) = (self.at(ax, ay), self.at(bx, by), self.at(x, y));
        self.out.push(PathCmd::CubicTo(ax, ay, bx, by, x, y));
    }

    fn close(&mut self) {
        self.out.push(PathCmd::Close);
    }
}

/// The glyph outlines of `run` filled with `gradient`, or `None` for a paint paths cannot draw.
pub(super) fn glyph_fill(run: &GlyphRun, gradient: &Gradient) -> Option<DrawPath> {
    let (paint, squash, pivot) = path_paint(&gradient.normalized())?;
    let outline = outlines(run, squash, pivot);
    if outline.is_empty() {
        return None;
    }
    let transform: Transform = [[1.0, 0.0, 0.0], [0.0, 1.0 / squash, pivot - pivot / squash]];
    let key = outline_key(crate::fnv::mix(0x7478_7466, u64::from(squash.to_bits())), &outline);
    let mut draw = DrawPath::new(Arc::from(outline), transform, key).with_fill(FillRule::NonZero, paint);
    draw.node = run.node;
    draw.opacity = run.opacity;
    Some(draw)
}

/// The color a host that cannot fill glyphs with a gradient draws the text in.
pub(super) fn flat_color(gradient: &Gradient) -> crate::style::Rgba8 {
    color_at(&gradient.normalized().stops, 0.5)
}

fn path_paint(gradient: &Gradient) -> Option<(PathPaint, f32, f32)> {
    let stops = gradient.stops.clone();
    match gradient.kind {
        GradientKind::Linear { start, end } => {
            Some((PathPaint::Linear(LinearGradient::new(start, end, stops)), 1.0, 0.0))
        }
        GradientKind::Radial { center, radius } => {
            let squash = radius[0] / radius[1].max(f32::EPSILON);
            let paint = PathPaint::Radial {
                center,
                radius: radius[0],
                stops,
            };
            Some((paint, squash, center.y))
        }
        GradientKind::Conic { .. } => None,
    }
}

fn outlines(run: &GlyphRun, squash: f32, pivot: f32) -> Vec<PathCmd> {
    let mut out = Vec::new();
    let Some(paragraph) = run.paragraph() else {
        return out;
    };
    let glyphs = run.positioned_glyphs();
    let origin = run.rect.origin;
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
            let coords: Vec<NormalizedCoord> = g
                .variation_coords
                .iter()
                .map(|&c| NormalizedCoord::from_bits(c))
                .collect();
            let mut pen = Pen {
                out: &mut out,
                x: origin.x + g.x,
                y: origin.y + g.y,
                skew: g.skew,
                squash,
                pivot,
            };
            let settings = DrawSettings::unhinted(Size::new(g.size), LocationRef::new(&coords));
            let _ = glyph.draw(settings, &mut pen);
        }
    });
    out
}

