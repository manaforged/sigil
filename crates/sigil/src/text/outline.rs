use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlineGlyph, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use smallvec::SmallVec;

use crate::hal::text::RenderedGlyph;
use crate::text::Paragraph;

pub(crate) fn coords(raw: &[i16]) -> SmallVec<[NormalizedCoord; 4]> {
    raw.iter().map(|&c| NormalizedCoord::from_bits(c)).collect()
}

pub(crate) fn for_each_outline(
    paragraph: &Paragraph,
    glyphs: &[RenderedGlyph],
    mut f: impl FnMut(&RenderedGlyph, &OutlineGlyph<'_>, DrawSettings<'_>),
) {
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
            let coords = coords(&g.variation_coords);
            f(
                g,
                &glyph,
                DrawSettings::unhinted(Size::new(g.size), LocationRef::new(&coords)),
            );
        }
    });
}

pub(crate) struct Oblique<'a, P: OutlinePen> {
    pen: &'a mut P,
    skew: f32,
}

impl<'a, P: OutlinePen> Oblique<'a, P> {
    pub(crate) const fn new(pen: &'a mut P, skew: f32) -> Self {
        Self { pen, skew }
    }

    const fn x(&self, x: f32, y: f32) -> f32 {
        self.skew.mul_add(y, x)
    }
}

impl<P: OutlinePen> OutlinePen for Oblique<'_, P> {
    fn move_to(&mut self, x: f32, y: f32) {
        let x = self.x(x, y);
        self.pen.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let x = self.x(x, y);
        self.pen.line_to(x, y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, x) = (self.x(cx, cy), self.x(x, y));
        self.pen.quad_to(cx, cy, x, y);
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        let (c0x, c1x, x) = (self.x(c0x, c0y), self.x(c1x, c1y), self.x(x, y));
        self.pen.curve_to(c0x, c0y, c1x, c1y, x, y);
    }

    fn close(&mut self) {
        self.pen.close();
    }
}
