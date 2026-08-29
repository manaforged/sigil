#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use skrifa::bitmap::BitmapData;
use skrifa::instance::Size;
use skrifa::{FontRef, GlyphId, MetadataProvider};

#[derive(Clone, Debug, PartialEq)]
pub struct ColorGlyphRaster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub ppem: f32,
}

#[must_use]
pub fn has_color_bitmaps(font: &FontRef<'_>) -> bool {
    !font.bitmap_strikes().is_empty()
}

#[must_use]
pub fn glyph_for_char(font: &FontRef<'_>, ch: char) -> Option<GlyphId> {
    font.charmap().map(ch)
}

#[must_use]
pub fn rasterize(font: &FontRef<'_>, gid: GlyphId, px_per_em: f32) -> Option<ColorGlyphRaster> {
    let glyph = font
        .bitmap_strikes()
        .glyph_for_size(Size::new(px_per_em), gid)?;
    let rgba = match glyph.data {
        BitmapData::Png(bytes) => decode_png(bytes)?,
        BitmapData::Bgra(bytes) => bgra_to_rgba(bytes, glyph.width, glyph.height)?,
        BitmapData::Mask(_) => return None,
    };
    Some(ColorGlyphRaster {
        width: glyph.width,
        height: glyph.height,
        rgba,
        ppem: glyph.ppem_y.max(1.0),
    })
}

fn decode_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    Some(decoded.to_rgba8().into_raw())
}

#[allow(
    clippy::many_single_char_names,
    reason = "intentional, reviewed trade-off"
)]
fn bgra_to_rgba(bytes: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let count = (w as usize).checked_mul(h as usize)?.checked_mul(4)?;
    if bytes.len() < count {
        return None;
    }
    let mut out = vec![0u8; count];
    for (dst, src) in out
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(bytes.as_chunks::<4>().0)
    {
        let &[b, g, r, a] = src;
        let &mut [ref mut dr, ref mut dg, ref mut db, ref mut da] = dst;
        *dr = unpremultiply(r, a);
        *dg = unpremultiply(g, a);
        *db = unpremultiply(b, a);
        *da = a;
    }
    Some(out)
}

fn unpremultiply(c: u8, a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let value = (u16::from(c) * 255 + u16::from(a) / 2) / u16::from(a);
    value.min(255) as u8
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "color_glyph_tests.rs"]
mod tests;
