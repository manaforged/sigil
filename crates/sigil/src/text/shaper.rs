#![allow(clippy::redundant_pub_crate, clippy::missing_const_for_fn)]

use std::sync::Arc;
#[cfg(feature = "shaping-complex")]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "shaping-complex")]
use harfrust::{FontRef as HrFontRef, ShaperData};

use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::charmap::Charmap;
use skrifa::instance::{Location, LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::raw::types::Tag;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::glyph::{GlyphId, GlyphPos, GlyphRun};

#[path = "shaper/classify.rs"]
mod classify_mod;
#[path = "shaper/variations.rs"]
mod variations;

#[cfg(not(feature = "shaping-complex"))]
use classify_mod::warn_complex_without_feature;
#[allow(unused_imports, reason = "intentional, reviewed trade-off")]
pub(crate) use classify_mod::{Script, classify_segment_script};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FeatureTag(pub [u8; 4]);

#[derive(Clone)]
pub struct Font {
    data: Arc<Vec<u8>>,
    index: u32,
    id: u64,
    variations: variations::VariationModel,
    #[cfg(feature = "shaping-complex")]
    tables: Arc<OnceLock<ShaperData>>,
}

impl core::fmt::Debug for Font {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Font")
            .field("len", &self.data.len())
            .field("index", &self.index)
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

static NEXT_FONT_ID: AtomicU64 = AtomicU64::new(1);

impl Font {
    pub fn from_bytes(data: Vec<u8>, index: u32) -> Result<Self, ShaperError> {
        Self::from_shared(Arc::new(data), index)
    }

    pub(crate) fn from_shared(data: Arc<Vec<u8>>, index: u32) -> Result<Self, ShaperError> {
        let font_ref = FontRef::from_index(&data, index).map_err(|_| ShaperError::InvalidFont)?;
        let variations = variations::VariationModel::new(&font_ref);
        Ok(Self {
            data,
            index,
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
            variations,
            #[cfg(feature = "shaping-complex")]
            tables: Arc::new(OnceLock::new()),
        })
    }

    #[cfg(feature = "shaping-complex")]
    pub(crate) fn tables(&self, font: &HrFontRef<'_>) -> &ShaperData {
        self.tables.get_or_init(|| ShaperData::new(font))
    }

    pub(crate) fn font_ref(&self) -> FontRef<'_> {
        FontRef::from_index(&self.data, self.index)
            .expect("Font::data validated in Font::from_bytes")
    }

    #[must_use]
    pub fn bytes_arc(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.data)
    }

    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.index
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn has_weight_axis(&self) -> bool {
        self.font_ref()
            .axes()
            .get_by_tag(Tag::new(b"wght"))
            .is_some()
    }

    pub(crate) fn location(&self, weight: u16, size_px: f32) -> Location {
        let settings = self.variation_settings(weight, size_px);
        self.font_ref()
            .axes()
            .location(settings.iter().map(|(tag, value)| (Tag::new(tag), *value)))
    }

    pub(crate) fn variation_settings(
        &self,
        weight: u16,
        size_px: f32,
    ) -> variations::VariationSettings {
        self.variations.settings(weight, size_px)
    }

    pub(crate) fn variation_coords(&self, weight: u16, size_px: f32) -> Box<[i16]> {
        self.location(weight, size_px)
            .coords()
            .iter()
            .map(|coord| coord.to_bits())
            .collect()
    }

    #[must_use]
    pub(crate) fn covers(&self, text: &str) -> bool {
        let charmap = self.font_ref().charmap();
        text.chars()
            .filter(|&ch| !is_default_ignorable(ch))
            .all(|ch| charmap.map(ch as u32).is_some())
    }
}

pub(crate) const fn is_default_ignorable(ch: char) -> bool {
    let cp = ch as u32;
    let plane = cp >> 16;
    if plane == 0 {
        match cp >> 8 {
            0x00 => cp == 0x00AD,
            0x03 => cp == 0x034F,
            0x06 => cp == 0x061C,
            0x17 => matches!(cp, 0x17B4..=0x17B5),
            0x18 => matches!(cp, 0x180B..=0x180F),
            0x20 => {
                matches!(cp, 0x200B..=0x200F)
                    || matches!(cp, 0x202A..=0x202E)
                    || matches!(cp, 0x2060..=0x206F)
            }
            0xFE => matches!(cp, 0xFE00..=0xFE0F) || cp == 0xFEFF,
            0xFF => matches!(cp, 0xFFF0..=0xFFF8),
            _ => false,
        }
    } else {
        match plane {
            0x01 => matches!(cp, 0x1D173..=0x1D17A),
            0x0E => matches!(cp, 0xE0000..=0xE0FFF),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TextSegment<'a> {
    pub(crate) text: &'a str,
    pub(crate) level: u8,
}

impl<'a> TextSegment<'a> {
    pub(crate) const fn at_level(text: &'a str, level: u8) -> Self {
        Self { text, level }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Style {
    pub(crate) size_px: f32,
    pub(crate) weight: u16,
    pub(crate) features: SmallVec<[FeatureTag; 4]>,
    pub(crate) letter_spacing_em: f32,
}

impl Style {
    pub(crate) fn regular(size_px: f32) -> Self {
        Self {
            size_px,
            weight: 400,
            features: SmallVec::new(),
            letter_spacing_em: 0.0,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::regular(16.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaperError {
    InvalidFont,
}

impl core::fmt::Display for ShaperError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidFont => f.write_str("invalid font data"),
        }
    }
}

impl std::error::Error for ShaperError {}

pub(crate) struct Shaper<'a> {
    font_ref: FontRef<'a>,
    font: &'a Font,
}

impl<'a> Shaper<'a> {
    pub(crate) fn new(font: &'a Font) -> Self {
        Self {
            font_ref: font.font_ref(),
            font,
        }
    }

    pub(crate) fn shape(&self, segment: &TextSegment<'_>, style: &Style) -> GlyphRun {
        if segment.text.is_empty() {
            return GlyphRun::empty();
        }
        #[cfg(feature = "shaping-complex")]
        {
            if let Some(run) = crate::text::shaping_complex::shape(segment, style, self.font) {
                return run;
            }
        }
        #[cfg(not(feature = "shaping-complex"))]
        {
            let script = classify_segment_script(segment);
            if script.is_complex() {
                warn_complex_without_feature(script);
            }
        }
        shape_simple(self.font, segment, style)
    }

    pub(crate) fn strut(&self, style: &Style) -> (f32, f32) {
        let size = Size::new(style.size_px.max(1.0));
        let location = self.font.location(style.weight, style.size_px);
        let m = self
            .font_ref
            .metrics(size, LocationRef::new(location.coords()));
        (m.ascent, -m.descent)
    }
}

fn shape_simple(font: &Font, segment: &TextSegment<'_>, style: &Style) -> GlyphRun {
    let font_ref = font.font_ref();
    let charmap = font_ref.charmap();
    let size = Size::new(style.size_px.max(1.0));
    let location = font.location(style.weight, style.size_px);
    let location_ref = LocationRef::new(location.coords());
    let metrics = GlyphMetrics::new(&font_ref, size, location_ref);
    let face_metrics = font_ref.metrics(size, location_ref);

    let text = segment.text;
    let mut glyphs: SmallVec<[GlyphPos; 16]> = SmallVec::with_capacity(text.len() / 2 + 1);
    let mut pen_x: f32 = 0.0;
    let mut min_x: f32 = 0.0;
    let mut max_x: f32 = 0.0;

    for (cluster_byte, grapheme) in text.grapheme_indices(true) {
        let cluster_advance = grapheme_advance(grapheme, &charmap, &metrics);
        emit_grapheme(
            grapheme,
            cluster_byte,
            &charmap,
            cluster_advance,
            &mut glyphs,
        );
        pen_x += cluster_advance;
        if pen_x > max_x {
            max_x = pen_x;
        }
        if pen_x < min_x {
            min_x = pen_x;
        }
    }

    let baseline = face_metrics.ascent;
    let width = max_x - min_x;

    GlyphRun {
        glyphs,
        baseline,
        width,
    }
}

fn grapheme_advance(grapheme: &str, charmap: &Charmap<'_>, metrics: &GlyphMetrics<'_>) -> f32 {
    let mut chars = grapheme.chars();
    let Some(first) = chars.next() else {
        return 0.0;
    };
    let gid = charmap.map(first as u32).unwrap_or_default();
    metrics.advance_width(gid).unwrap_or(0.0)
}

fn emit_grapheme(
    grapheme: &str,
    cluster_byte: usize,
    charmap: &Charmap<'_>,
    cluster_advance: f32,
    out: &mut SmallVec<[GlyphPos; 16]>,
) {
    let cluster = u32::try_from(cluster_byte).unwrap_or(u32::MAX);
    for (idx, ch) in grapheme.chars().enumerate() {
        let skrifa_gid = charmap.map(ch as u32).unwrap_or_default();
        let id_u16 = u16::try_from(skrifa_gid.to_u32()).unwrap_or(0);
        let advance = if idx == 0 { cluster_advance } else { 0.0 };
        out.push(GlyphPos {
            id: GlyphId(id_u16),
            advance,
            offset_x: 0.0,
            offset_y: 0.0,
            cluster,
        });
    }
}

#[cfg(test)]
#[path = "shaper/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shaper/covers_tests.rs"]
mod covers_tests;
