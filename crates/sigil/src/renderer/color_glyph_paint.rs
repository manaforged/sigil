use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::hal::{Point, Rect, RenderedGlyph};
use crate::kernel::{ImageSource, NodeId, ObjectFit};
use crate::text::{Paragraph, color_glyph};
use skrifa::{FontRef, GlyphId};

use crate::renderer::scene_command::DrawImage;
use crate::scene::builder::SceneBuilder;

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

type ColorFontEntry = (Arc<Vec<u8>>, u32);
type ColorFont = Option<ColorFontEntry>;
type ColorFonts = HashMap<u64, ColorFontEntry>;
type RasterCacheMap = HashMap<(u64, u32, u32, [u32; 4]), Option<ImageSource>>;

const RASTER_CAP: usize = 1_024;

thread_local! {
    static FONTS: RefCell<HashMap<u64, ColorFont>> = RefCell::new(HashMap::new());
    static RASTERS: RefCell<RasterCacheMap> = RefCell::new(HashMap::new());
}

fn collect_color_fonts(para: &Paragraph) -> ColorFonts {
    let mut color_fonts = HashMap::new();
    FONTS.with(|cell| {
        let mut fonts = cell.borrow_mut();
        para.for_each_font_arc(|font_id, bytes, index| {
            let entry = fonts
                .entry(font_id)
                .or_insert_with(|| detect_color_font(bytes, index));
            if let Some(found) = entry {
                color_fonts.insert(font_id, found.clone());
            }
        });
    });
    color_fonts
}

fn detect_color_font(bytes: &Arc<Vec<u8>>, index: u32) -> ColorFont {
    FontRef::from_index(bytes, index)
        .is_ok_and(|font| color_glyph::has_color_glyphs(&font))
        .then(|| (Arc::clone(bytes), index))
}

fn cached_raster(glyph: &RenderedGlyph, bytes: &[u8], index: u32) -> Option<ImageSource> {
    let color = glyph.color;
    let key = (
        glyph.font_id,
        glyph.glyph_id,
        glyph.size.to_bits(),
        [
            color.r.to_bits(),
            color.g.to_bits(),
            color.b.to_bits(),
            color.a.to_bits(),
        ],
    );
    RASTERS.with(|cell| {
        let mut rasters = cell.borrow_mut();
        if let Some(found) = rasters.get(&key) {
            return found.clone();
        }
        if rasters.len() >= RASTER_CAP {
            rasters.clear();
        }
        let source = rasterize(glyph, bytes, index);
        rasters.insert(key, source.clone());
        source
    })
}

fn rasterize(glyph: &RenderedGlyph, bytes: &[u8], index: u32) -> Option<ImageSource> {
    let font = FontRef::from_index(bytes, index).ok()?;
    let raster =
        color_glyph::rasterize(&font, GlyphId::new(glyph.glyph_id), glyph.size, glyph.color)?;
    ImageSource::from_rgba8(raster.width, raster.height, raster.rgba)
}

fn emit_one(
    scene: &mut SceneBuilder,
    target: &GlyphTarget,
    glyph: &RenderedGlyph,
    color_fonts: &ColorFonts,
) {
    let Some((bytes, index)) = color_fonts.get(&glyph.font_id) else {
        return;
    };
    let Some(source) = cached_raster(glyph, bytes, *index) else {
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
        crop: None,
        raster: None,
    });
}
