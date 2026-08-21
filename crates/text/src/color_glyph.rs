//! Color-glyph (emoji) rasterization.
//!
//! Emoji carry per-glyph color art the `Slug` outline pipeline cannot
//! express. Noto Color Emoji stores 128px PNG bitmaps in `CBDT`/`CBLC`;
//! Apple Color Emoji uses `sbix`. This module decodes the nearest strike
//! for a glyph into straight-alpha RGBA8 so the renderer can upload it to
//! its full-color `PolySprite` atlas. The `Slug` path stays monochrome;
//! anything with a color bitmap routes here instead.

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use skrifa::bitmap::BitmapData;
use skrifa::instance::Size;
use skrifa::{FontRef, GlyphId, MetadataProvider};

/// A rasterized color glyph plus the strike size it was authored at.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorGlyphRaster {
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Straight-alpha RGBA8, row-major, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
    /// Pixels-per-em the strike was authored at: the bitmap spans one em
    /// at this size, so the caller scales it by `font_px / ppem` to place
    /// it against the pen origin.
    pub ppem: f32,
}

/// True when `font` carries color bitmap strikes (i.e. it is an emoji
/// font like Noto Color Emoji or Apple Color Emoji).
#[must_use]
pub fn has_color_bitmaps(font: &FontRef<'_>) -> bool {
    !font.bitmap_strikes().is_empty()
}

/// Glyph id for a single scalar value in `font`'s cmap, or `None`.
#[must_use]
pub fn glyph_for_char(font: &FontRef<'_>, ch: char) -> Option<GlyphId> {
    font.charmap().map(ch)
}

/// Rasterize color glyph `gid` from `font` near `px_per_em`.
///
/// Returns `None` when the glyph has no color bitmap. skrifa picks the
/// nearest available strike; emoji fonts are usually authored at one large
/// size (128px for Noto), so the raster is scaled at paint time rather
/// than re-hinted.
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

/// Decode an embedded PNG strike (the CBDT/sbix 32-bit case) to RGBA8.
fn decode_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    Some(decoded.to_rgba8().into_raw())
}

/// Convert premultiplied BGRA (the rare uncompressed strike) to straight
/// RGBA8. Rows are byte-aligned 32bpp, so a flat swizzle suffices.
#[allow(clippy::many_single_char_names)]
fn bgra_to_rgba(bytes: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let count = (w as usize).checked_mul(h as usize)?.checked_mul(4)?;
    if bytes.len() < count {
        return None;
    }
    let mut out = vec![0u8; count];
    for (dst, src) in out.chunks_exact_mut(4).zip(bytes.chunks_exact(4)) {
        let &[b, g, r, a] = src else { continue };
        let &mut [ref mut dr, ref mut dg, ref mut db, ref mut da] = dst else {
            continue;
        };
        *dr = unpremultiply(r, a);
        *dg = unpremultiply(g, a);
        *db = unpremultiply(b, a);
        *da = a;
    }
    Some(out)
}

/// Recover a straight-alpha channel from a premultiplied one.
fn unpremultiply(c: u8, a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let value = (u16::from(c) * 255 + u16::from(a) / 2) / u16::from(a);
    value.min(255) as u8
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
#[path = "color_glyph_tests.rs"]
mod tests;
