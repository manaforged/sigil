//! `HarfRust`-backed shaping (G1.6; default shaper since Wave 4).
//!
//! Gated behind the `shaping-complex` cargo feature, which is on by
//! default. Shapes every script `Shaper::shape` dispatches to it —
//! Latin/Greek/Cyrillic get kerning, ligatures, and mark positioning
//! in addition to the Arabic/Hebrew/Devanagari/Khmer/Thai/Lao joining
//! and reordering it always handled. The owned skrifa shaper in
//! `shaper.rs` is the fallback for when this returns `None`.
//!
//! `HarfRust` shapes in font design units (`units_per_em`-relative);
//! we scale `x_advance` / `x_offset` to user pixels using the
//! standard `size_px / upem` factor before populating `GlyphRun`.
//!
//! The entry point [`shape`] returns `None` if the font fails to
//! parse with `HarfRust` or the shaped buffer comes back empty —
//! callers fall through to the owned shaper in that case.

#![allow(clippy::redundant_pub_crate)]

use harfrust::{
    Direction, FontRef as HrFontRef, GlyphBuffer, Script as HrScript, ShaperData, UnicodeBuffer,
    script as hr_script,
};
use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::instance::{LocationRef, Size};
use smallvec::SmallVec;

use crate::glyph::{GlyphId, GlyphPos, GlyphRun, Rect};
use crate::shaper::{Font, Script, Style, TextSegment, classify_segment_script};

/// Shape `segment` via `HarfRust` under `style`.
///
/// Returns `None` when `HarfRust` cannot bind to `font` (malformed
/// face) or produces an empty glyph buffer; callers fall back to
/// the owned shaper.
pub(crate) fn shape(segment: &TextSegment<'_>, style: &Style, font: &Font) -> Option<GlyphRun> {
    if segment.text.is_empty() {
        return Some(GlyphRun::empty());
    }
    let bytes = font.bytes_arc();
    let hr_font = HrFontRef::from_index(bytes.as_slice(), font.face_index()).ok()?;
    let data = ShaperData::new(&hr_font);
    let hr_shaper = data
        .shaper(&hr_font)
        .point_size(Some(style.size_px))
        .build();

    let script = classify_segment_script(segment);
    let hr_script = map_script(script);
    let direction = if segment.level & 1 == 1 {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    };

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(segment.text);
    buffer.set_direction(direction);
    buffer.set_script(hr_script);

    let features = build_features(style);
    let glyphs_buf = hr_shaper.shape(buffer, &features);
    if glyphs_buf.is_empty() {
        return None;
    }

    // `HarfRust` returns metrics in font units; we need a per-pixel
    // scale factor and we also need the face metrics for baseline +
    // bbox, both of which we still source from skrifa.
    let skrifa_font = FontRef::from_index(bytes.as_slice(), font.face_index()).ok()?;
    let size = Size::new(style.size_px.max(1.0));
    let face_metrics = skrifa_font.metrics(size, LocationRef::default());
    let upem = f32::from(face_metrics.units_per_em);
    let scale = if upem > 0.0 {
        style.size_px / upem
    } else {
        1.0
    };

    Some(build_glyph_run(&glyphs_buf, scale, &face_metrics))
}

/// Map our internal `Script` to `HarfRust`'s iso15924 `Script`.
///
/// Since Wave 4 this feeds every segment (not just the scripts that
/// strictly need GSUB/GPOS to render) — Latin and Cjk need their own
/// tags so `HarfRust` selects the font's per-script `kern`/`liga`
/// lookups (many fonts key those under `latn`/`hani`, not `DFLT`).
const fn map_script(script: Script) -> HrScript {
    match script {
        Script::Latin => hr_script::LATIN,
        // ponytail: Han tag for all CJK; Hiragana/Katakana/Hangul get
        // their own iso15924 tags too, but the owned Script enum
        // doesn't split them out yet — split when vertical/CJK-
        // specific GSUB features need it.
        Script::Cjk => hr_script::HAN,
        Script::Arabic => hr_script::ARABIC,
        Script::Hebrew => hr_script::HEBREW,
        Script::Devanagari => hr_script::DEVANAGARI,
        Script::Khmer => hr_script::KHMER,
        Script::Thai => hr_script::THAI,
        Script::Lao => hr_script::LAO,
        // Common/Other: let `HarfRust` inspect text-derived properties.
        Script::Common | Script::Other => hr_script::COMMON,
    }
}

/// Convert `style.features` (4-byte OT tags) to `HarfRust` globally-
/// applied feature settings (value=1, range=..) per `HarfBuzz` norm.
fn build_features(style: &Style) -> SmallVec<[harfrust::Feature; 4]> {
    style
        .features
        .iter()
        .map(|tag| harfrust::Feature::new(harfrust::Tag::new(&tag.0), 1, ..))
        .collect()
}

/// Translate the `HarfRust` glyph buffer into our `GlyphRun`.
///
/// Allow `cast_precision_loss`: `HarfRust` emits glyph metrics as
/// `i32` font-design units. In practice these fit in a 16-bit range
/// for any conforming OpenType font (`units_per_em` is `u16`), so
/// the conversion to `f32` is lossless for valid input.
#[allow(clippy::cast_precision_loss)]
fn build_glyph_run(
    glyphs: &GlyphBuffer,
    scale: f32,
    face_metrics: &skrifa::metrics::Metrics,
) -> GlyphRun {
    let infos = glyphs.glyph_infos();
    let positions = glyphs.glyph_positions();
    let mut out: SmallVec<[GlyphPos; 16]> = SmallVec::with_capacity(infos.len());
    let mut pen_x: f32 = 0.0;
    let mut min_x: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    for (info, pos) in infos.iter().zip(positions.iter()) {
        let advance = pos.x_advance as f32 * scale;
        let offset_x = pos.x_offset as f32 * scale;
        let offset_y = pos.y_offset as f32 * scale;
        let id_u16 = u16::try_from(info.glyph_id).unwrap_or(0);
        out.push(GlyphPos {
            id: GlyphId(id_u16),
            advance,
            offset_x,
            offset_y,
            cluster: info.cluster,
        });
        pen_x += advance;
        if pen_x > max_x {
            max_x = pen_x;
        }
        if pen_x < min_x {
            min_x = pen_x;
        }
    }
    let baseline = face_metrics.ascent;
    let bbox = Rect {
        x: min_x,
        y: -face_metrics.ascent,
        w: max_x - min_x,
        h: face_metrics.ascent - face_metrics.descent,
    };
    GlyphRun {
        glyphs: out,
        baseline,
        bbox,
    }
}

#[cfg(test)]
#[path = "shaping_complex/tests.rs"]
mod tests;
