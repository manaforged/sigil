use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use ahash::{AHashMap, AHasher};
use skrifa::MetadataProvider;
use skrifa::instance::{LocationRef, Size};
use smallvec::SmallVec;

use crate::text::glyph::{GlyphPos, ShapedRun};
use crate::text::shaper::{FaceStyle, Font, TextSegment};

const SIMPLE: &[u8] = b"0123456789 .,-+%$:/()";
const MAX_ENTRIES: usize = 4_096;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct NumericKey {
    font: u64,
    index: u32,
    size: u32,
    weight: u16,
    features: u64,
    ch: u8,
}

#[derive(Clone, Copy)]
struct GlyphEntry {
    glyph: GlyphPos,
    ascent: f32,
}

thread_local! {
    static CACHE: RefCell<AHashMap<NumericKey, Option<GlyphEntry>>> = RefCell::new(AHashMap::new());
}

pub(crate) fn is_simple(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| SIMPLE.contains(&b))
}

pub(crate) fn shape(
    segment: &TextSegment<'_>,
    style: &FaceStyle,
    font: &Font,
    full: impl Fn(&TextSegment<'_>, &FaceStyle, &Font) -> Option<ShapedRun>,
) -> Option<ShapedRun> {
    if segment.level & 1 == 1 || !is_simple(segment.text) {
        return None;
    }
    let mut features = AHasher::default();
    for tag in &style.features {
        tag.0.hash(&mut features);
    }
    let base = NumericKey {
        font: font.id(),
        index: font.face_index(),
        size: style.size_px.to_bits(),
        weight: style.weight,
        features: features.finish(),
        ch: 0,
    };
    let mut glyphs: SmallVec<[GlyphPos; 16]> = SmallVec::with_capacity(segment.text.len());
    let mut pen = 0.0_f32;
    let mut ascent = 0.0_f32;
    for (offset, ch) in segment.text.bytes().enumerate() {
        let key = NumericKey { ch, ..base };
        let entry = CACHE.with(|cache| {
            if let Some(hit) = cache.borrow().get(&key) {
                return *hit;
            }
            let entry = shape_char(ch, segment.level, style, font, &full);
            let mut cache = cache.borrow_mut();
            if cache.len() >= MAX_ENTRIES {
                cache.clear();
            }
            cache.insert(key, entry);
            entry
        })?;
        let mut glyph = entry.glyph;
        glyph.cluster = u32::try_from(offset).unwrap_or(u32::MAX);
        pen += glyph.advance;
        glyphs.push(glyph);
        ascent = entry.ascent;
    }
    Some(ShapedRun {
        glyphs,
        baseline: ascent,
        width: pen,
    })
}

fn shape_char(
    ch: u8,
    level: u8,
    style: &FaceStyle,
    font: &Font,
    full: &impl Fn(&TextSegment<'_>, &FaceStyle, &Font) -> Option<ShapedRun>,
) -> Option<GlyphEntry> {
    let text = [ch];
    let text = core::str::from_utf8(&text).ok()?;
    let run = full(&TextSegment::at_level(text, level), style, font)?;
    let [glyph] = run.glyphs.as_slice() else {
        return None;
    };
    let face = font.font_ref();
    let location = font.location(style.weight, style.size_px);
    let metrics = face.metrics(
        Size::new(style.size_px.max(1.0)),
        LocationRef::new(location.coords()),
    );
    Some(GlyphEntry {
        glyph: *glyph,
        ascent: metrics.ascent,
    })
}

#[cfg(test)]
#[path = "numeric_run_tests.rs"]
mod tests;
