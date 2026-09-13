use std::collections::HashMap;

use crate::hal::{Point, Rect, RenderedGlyph};
use crate::kernel::{ImageSource, NodeId, ObjectFit};
use crate::text::{Paragraph, color_glyph};
use skrifa::{FontRef, GlyphId};

use crate::renderer::scene_builder::SceneBuilder;
use crate::renderer::scene_command::DrawImage;

const EMOJI_ASCENT_FRAC: f32 = 0.85;

struct GlyphTarget {
    id: NodeId,
    rect: Rect,
    opacity: u16,
}

pub(crate) fn emit_color_glyphs(
    scene: &mut SceneBuilder,
    id: NodeId,
    rect: Rect,
    para: &Paragraph,
    opacity: u16,
) {
    let color_fonts = collect_color_fonts(para);
    if color_fonts.is_empty() {
        return;
    }
    let target = GlyphTarget { id, rect, opacity };
    for glyph in para.rendered_glyphs() {
        emit_one(scene, &target, &glyph, &color_fonts);
    }
}

fn collect_color_fonts(para: &Paragraph) -> HashMap<u64, (Vec<u8>, u32)> {
    let mut color_fonts: HashMap<u64, (Vec<u8>, u32)> = HashMap::new();
    para.for_each_font(|font_id, bytes, index| {
        let is_color = FontRef::from_index(bytes, index)
            .is_ok_and(|font| color_glyph::has_color_bitmaps(&font));
        if is_color {
            color_fonts.insert(font_id, (bytes.to_vec(), index));
        }
    });
    color_fonts
}

fn emit_one(
    scene: &mut SceneBuilder,
    target: &GlyphTarget,
    glyph: &RenderedGlyph,
    color_fonts: &HashMap<u64, (Vec<u8>, u32)>,
) {
    let Some((bytes, index)) = color_fonts.get(&glyph.font_id) else {
        return;
    };
    let Ok(font) = FontRef::from_index(bytes, *index) else {
        return;
    };
    let Some(raster) = color_glyph::rasterize(&font, GlyphId::new(glyph.glyph_id), glyph.size)
    else {
        return;
    };
    let Some(source) = ImageSource::from_rgba8(raster.width, raster.height, raster.rgba) else {
        return;
    };
    let em = glyph.size;
    let destination = Rect::new(
        Point::new(
            target.rect.origin.x + glyph.x,
            em.mul_add(-EMOJI_ASCENT_FRAC, target.rect.origin.y + glyph.y),
        ),
        em,
        em,
    );
    scene.draw_image(DrawImage {
        node: Some(target.id),
        source,
        fit: ObjectFit::Contain,
        destination,
        opacity: target.opacity,
    });
}
