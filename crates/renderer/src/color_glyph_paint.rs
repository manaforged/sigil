//! Color-glyph (emoji) emission for the paint pass.
//!
//! The `Slug` outline pipeline skips glyphs with no outline — exactly the
//! `CBDT`/`sbix` color-bitmap emoji case, so emoji would otherwise vanish.
//! This module rasterizes those glyphs (via [`sigil_text::color_glyph`])
//! and emits them as `DrawImage` commands, reusing the renderer's existing
//! RGBA image path, so emoji paint in full color.

use std::collections::HashMap;

use sigil_hal::{Point, Rect, RenderedGlyph};
use sigil_kernel::{ImageSource, NodeId, ObjectFit};
use sigil_text::{Paragraph, color_glyph};
use skrifa::{FontRef, GlyphId};

use crate::scene_builder::SceneBuilder;
use crate::scene_command::DrawImage;

/// Fraction of the em the emoji box rises above the baseline. Noto Color
/// Emoji art sits roughly cap-height to a shallow descent; 0.85 up leaves
/// 0.15 below so the glyph centres on the text without visible clipping.
const EMOJI_ASCENT_FRAC: f32 = 0.85;

/// Node-paint context shared by every color glyph in one text node.
struct GlyphTarget {
    id: NodeId,
    rect: Rect,
    opacity: u16,
}

/// Emit a `DrawImage` for every color-bitmap glyph in `para`, positioned
/// against the node's `rect`.
///
/// Mono glyphs are untouched: they render through the `Slug` path, which
/// already skips these outline-less glyphs, so there is no double paint.
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

/// Map each color-bitmap font id in the paragraph to owned bytes, so a
/// glyph can be rasterized after the `for_each_font` borrow ends.
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

/// Rasterize one glyph and, if it has a color bitmap, push its `DrawImage`.
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
