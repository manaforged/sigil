use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::metrics::{BoundingBox, GlyphMetrics};
use skrifa::{FontRef, GlyphId};

use crate::geometry::{Point, Rect};

use super::{FaceEntry, LayoutLine, LineGlyph, Paragraph};

impl Paragraph {
    pub(crate) fn ink_rect(&self) -> Option<Rect> {
        *self.inner.ink.get_or_init(|| self.measure_ink())
    }

    fn measure_ink(&self) -> Option<Rect> {
        let size = Size::new(self.inner.font_size.max(1.0));
        let mut ink: Option<Rect> = None;
        for (slot, face) in self.inner.fonts.iter().enumerate() {
            let font = FontRef::from_index(&face.bytes, face.index).ok();
            let coords: Vec<NormalizedCoord> = face
                .variation_coords
                .iter()
                .map(|&coord| NormalizedCoord::from_bits(coord))
                .collect();
            let metrics = font
                .as_ref()
                .map(|font| GlyphMetrics::new(font, size, LocationRef::new(&coords)));
            for line in &self.inner.lines {
                for glyph in line
                    .glyphs
                    .iter()
                    .filter(|g| usize::from(g.face_slot) == slot)
                {
                    let bounds = metrics
                        .as_ref()
                        .and_then(|m| m.bounds(GlyphId::new(glyph.glyph_id)));
                    if let Some(rect) = glyph_ink(line, glyph, face, bounds) {
                        ink = Some(ink.map_or(rect, |so_far| so_far.union(rect)));
                    }
                }
            }
        }
        ink
    }
}

fn glyph_ink(
    line: &LayoutLine,
    glyph: &LineGlyph,
    face: &FaceEntry,
    bounds: Option<BoundingBox>,
) -> Option<Rect> {
    let Some(b) = bounds else {
        return Some(Rect::new(
            Point::new(glyph.x, line.line_top()),
            glyph.advance.max(0.0),
            line.height,
        ));
    };
    if b.x_max <= b.x_min || b.y_max <= b.y_min {
        return None;
    }
    let lean = [face.skew * b.y_min, face.skew * b.y_max];
    let x0 = glyph.x + b.x_min + lean[0].min(lean[1]);
    let x1 = glyph.x + b.x_max + lean[0].max(lean[1]);
    let baseline = line.baseline_y + glyph.offset_y;
    Some(Rect::new(
        Point::new(x0, baseline - b.y_max),
        x1 - x0,
        b.y_max - b.y_min,
    ))
}
