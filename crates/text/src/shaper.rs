//! Owned shaper — skrifa-backed glyph layout (DR-044, G1.3).
//!
//! Per-grapheme glyph lookup via `skrifa::charmap::Charmap`; advances
//! from `skrifa::metrics::GlyphMetrics`. Fallback path: since Wave 4,
//! `HarfRust` (`shaping_complex.rs`) is the default shaper for every
//! script (kerning/ligatures/marks) behind the crate-default
//! `shaping-complex` feature. This module only runs when `HarfRust`
//! returns nothing or the feature is off — still Latin, CJK
//! (ideographs + `U+3000..U+303F` / `U+FF00..U+FFEF` punctuation),
//! and emoji passthrough (G1.5 atlas), just no kerning/ligatures.
//!
//! `Shaper` is `Send + Sync` — skrifa lookups are read-only against
//! owned font data, so the engine can shape across rayon threads
//! without a pool.

// `redundant_pub_crate` fires because shaper is a `pub(crate) mod`;
// `missing_const_for_fn` fires on accessors that could be const.
#![allow(clippy::redundant_pub_crate, clippy::missing_const_for_fn)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::charmap::Charmap;
use skrifa::instance::{Location, LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::raw::types::Tag;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation;

use crate::glyph::{GlyphId, GlyphPos, GlyphRun, Rect};

#[path = "shaper/classify.rs"]
mod classify_mod;
#[path = "shaper/variations.rs"]
mod variations;

#[cfg(not(feature = "shaping-complex"))]
use classify_mod::warn_complex_without_feature;
// `Script` is re-exported for `shaping_complex` + the shaper tests, which
// keep importing from `crate::shaper::*` for API-surface stability after
// the G1.7 module split.
#[allow(unused_imports)]
pub(crate) use classify_mod::{Script, classify_segment_script};

/// OpenType-style feature tag (4 ASCII bytes, e.g. `*b"liga"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FeatureTag(pub [u8; 4]);

/// Owned font handle backing the skrifa lookup tables.
///
/// Inner bytes are reference-counted (`Arc<Vec<u8>>`) so paragraphs
/// produced by the engine can hold a cheap, lifetime-free handle to
/// the source bytes for the renderer's glyph-outline cache. Skrifa
/// parses lazily, so the construction cost is just table-directory
/// validation.
#[derive(Debug, Clone)]
pub struct Font {
    data: Arc<Vec<u8>>,
    index: u32,
    id: u64,
    variations: variations::VariationModel,
}

static NEXT_FONT_ID: AtomicU64 = AtomicU64::new(1);

impl Font {
    /// Construct a font from owned TTF/OTF bytes.
    ///
    /// `index` selects the face inside a TTC; pass `0` for single-face
    /// files.
    ///
    /// # Errors
    ///
    /// Returns `ShaperError::InvalidFont` when skrifa rejects the
    /// table directory.
    pub fn from_bytes(data: Vec<u8>, index: u32) -> Result<Self, ShaperError> {
        Self::from_shared(Arc::new(data), index)
    }

    /// Construct a font while sharing an already-owned sfnt blob.
    pub(crate) fn from_shared(data: Arc<Vec<u8>>, index: u32) -> Result<Self, ShaperError> {
        // Validate at construction so `font_ref()` never fails downstream.
        let font_ref = FontRef::from_index(&data, index).map_err(|_| ShaperError::InvalidFont)?;
        let variations = variations::VariationModel::new(&font_ref);
        Ok(Self {
            data,
            index,
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
            variations,
        })
    }

    fn font_ref(&self) -> FontRef<'_> {
        // Validated in `from_bytes`; this branch is unreachable in
        // practice but we surface a clean panic message rather than
        // `unwrap()` per rust-hygiene.
        FontRef::from_index(&self.data, self.index)
            .expect("Font::data validated in Font::from_bytes")
    }

    /// Cheap clone of the reference-counted font bytes.
    #[must_use]
    pub fn bytes_arc(&self) -> Arc<Vec<u8>> {
        Arc::clone(&self.data)
    }

    /// Face index inside a TTC (0 for single-face files).
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.index
    }

    /// Stable engine-assigned face identifier.
    ///
    /// Two clones share an id. Distinct faces in the same collection have
    /// distinct ids even though their sfnt bytes are shared.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Whether this face can resolve CSS weights through an OpenType `wght` axis.
    pub(crate) fn has_weight_axis(&self) -> bool {
        self.font_ref()
            .axes()
            .get_by_tag(Tag::new(b"wght"))
            .is_some()
    }

    /// Resolve weight and optical size into this face's normalized axis order.
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

    /// Normalized variation coordinates carried through renderer caches.
    pub(crate) fn variation_coords(&self, weight: u16, size_px: f32) -> Box<[i16]> {
        self.location(weight, size_px)
            .coords()
            .iter()
            .map(|coord| coord.to_bits())
            .collect()
    }

    /// Whether every non-default-ignorable codepoint in `text` has a
    /// cmap entry in this face.
    ///
    /// Used by the engine's per-run fallback chain (Wave 4) to pick the
    /// first registered face with full coverage for a run before
    /// falling back to `.notdef` tofu. A single cmap lookup per
    /// codepoint — cheap relative to a full shape.
    ///
    /// the reviewer adversarial review, finding 2: default-ignorable shaping
    /// controls (ZWJ, ZWNJ, VS15/VS16) commonly have no standalone cmap
    /// glyph — they're consumed during shaping (joining, variation
    /// selection), not drawn — so requiring one rejected faces that
    /// actually shape the surrounding text correctly. They're skipped
    /// here rather than checked.
    ///
    /// cmap presence alone still doesn't prove the SHAPED result avoids
    /// `.notdef` (a face can map a codepoint but fail to produce the
    /// substituted glyph a script's GSUB rules need) — the engine's
    /// [`shape_validated`](crate::engine) closes that gap by validating
    /// the actual shaped buffer and falling through the chain if this
    /// face's cmap coverage didn't pan out.
    #[must_use]
    pub(crate) fn covers(&self, text: &str) -> bool {
        let charmap = self.font_ref().charmap();
        text.chars()
            .filter(|&ch| !is_default_ignorable(ch))
            .all(|ch| charmap.map(ch as u32).is_some())
    }
}

/// Unicode's `Default_Ignorable_Code_Point` derived property —
/// shaping/formatting controls consumed during text processing rather
/// than drawn as a visible glyph (ZWJ/ZWNJ, variation selectors, bidi
/// formatting controls, Mongolian FVS, tag characters, etc). Codepoints
/// that commonly have no standalone cmap glyph but are NOT rejected
/// coverage failures.
///
/// Range table generated from `DerivedCoreProperties.txt`
/// (Unicode 14.0, `grep '; Default_Ignorable_Code_Point '`). No
/// crate already in this workspace's dependency graph exposes the
/// property publicly (checked `unicode-properties` 0.1.4, pulled in
/// transitively via `rustybuzz`: only `emoji`/`general_category`
/// modules, no `Default_Ignorable_Code_Point`; `skrifa`/`harfrust`
/// have no public surface for it — `harfrust`'s own
/// `is_default_ignorable` in `src/hb/unicode.rs` is `pub(crate)`).
///
/// Deliberately excludes 4 codepoints the raw property nominally
/// covers but that real fonts render as ordinary spacing glyphs
/// (U+115F/U+1160 Hangul Choseong/Jungseong Filler, U+3164 Hangul
/// Filler, U+FFA0 Halfwidth Hangul Filler) and the Shorthand Format
/// controls U+1BCA0..=U+1BCA3 — matching `HarfBuzz`'s documented
/// practical exceptions (`harfbuzz/harfbuzz#503`): a font missing one
/// of THOSE is a real, visible coverage gap, not an invisible control
/// safe to skip. Since `HarfRust` is this crate's shaping backend,
/// aligning with its own ignorable set keeps `covers`/`shape_validated`
/// consistent with what actually shapes the text.
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

// Skrifa's `FontRef` is `Send + Sync`; `Vec<u8>` is too. Marker is
// implicit via the field types — no manual impl needed. The whitebox
// test below pins the contract.

/// Segmented text input to the shaper.
///
/// `level` is the UAX #9 embedding level (even = LTR, odd = RTL).
/// G1.3 honours `level` only for cluster monotonicity assertions —
/// the visual reordering inside RTL runs is paragraph-level and lands
/// in G1.4 when the engine wires segments through the shaper.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextSegment<'a> {
    /// Source text — already split by bidi run + script run.
    pub(crate) text: &'a str,
    /// Bidi embedding level (UAX #9) for this segment.
    pub(crate) level: u8,
}

impl<'a> TextSegment<'a> {
    /// Construct an LTR segment over `text`.
    #[cfg(test)]
    pub(crate) const fn ltr(text: &'a str) -> Self {
        Self { text, level: 0 }
    }

    /// Construct a segment at an explicit UAX #9 embedding level.
    pub(crate) const fn at_level(text: &'a str, level: u8) -> Self {
        Self { text, level }
    }
}

/// Resolved text-style projection driving glyph metrics.
///
/// G1.4's engine populates this from `sigil_style::TextStyle`.
#[derive(Debug, Clone)]
pub(crate) struct Style {
    pub(crate) size_px: f32,
    pub(crate) weight: u16,
    pub(crate) features: SmallVec<[FeatureTag; 4]>,
    pub(crate) letter_spacing_em: f32,
}

impl Style {
    /// Construct a regular 16 px / weight-400 style with no features.
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

/// Errors surfaced by the shaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaperError {
    /// Font data did not parse as a valid OpenType/TrueType face.
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

/// Skrifa-backed shaper.
///
/// Borrows the [`Font`] for its lifetime; cheap to construct
/// (skrifa lookups are per-call zero-copy). Threading note: the
/// shaper is `Send + Sync`, so the engine can dispatch shaping
/// across rayon workers without a pool.
pub(crate) struct Shaper<'a> {
    font_ref: FontRef<'a>,
    font: &'a Font,
}

impl<'a> Shaper<'a> {
    /// Construct a shaper bound to `font`.
    pub(crate) fn new(font: &'a Font) -> Self {
        Self {
            font_ref: font.font_ref(),
            font,
        }
    }

    /// Shape `segment` under `style`, producing a [`GlyphRun`].
    ///
    /// Every script routes through `HarfRust` when `shaping-complex`
    /// is on (the crate default). `shape_simple` is the fallback when
    /// `HarfRust` returns nothing or the feature is off, with a
    /// one-shot per-script warning for complex scripts in that case.
    pub(crate) fn shape(&self, segment: &TextSegment<'_>, style: &Style) -> GlyphRun {
        if segment.text.is_empty() {
            return GlyphRun::empty();
        }
        #[cfg(feature = "shaping-complex")]
        {
            if let Some(run) = crate::shaping_complex::shape(segment, style, self.font) {
                return run;
            }
            // `HarfRust` returned nothing; fall through to the owned
            // shaper rather than emit nothing.
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

    /// Face ascent + descent (both positive, px at `style.size_px`) —
    /// the paragraph strut that anchors every line's baseline.
    pub(crate) fn strut(&self, style: &Style) -> (f32, f32) {
        let size = Size::new(style.size_px.max(1.0));
        let location = self.font.location(style.weight, style.size_px);
        let m = self
            .font_ref
            .metrics(size, LocationRef::new(location.coords()));
        // skrifa reports descent as negative (below baseline).
        (m.ascent, -m.descent)
    }
}

/// Shape `segment` along the simple (Latin/CJK/emoji) path.
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
    let bbox = Rect {
        x: min_x,
        y: -face_metrics.ascent,
        w: max_x - min_x,
        h: face_metrics.ascent - face_metrics.descent,
    };

    GlyphRun {
        glyphs,
        baseline,
        bbox,
    }
}

/// Total horizontal advance of a single grapheme cluster.
///
/// For multi-codepoint clusters (ZWJ emoji, combining marks) the
/// advance is taken from the first base glyph; combining marks have
/// zero advance per OpenType convention.
fn grapheme_advance(grapheme: &str, charmap: &Charmap<'_>, metrics: &GlyphMetrics<'_>) -> f32 {
    let mut chars = grapheme.chars();
    let Some(first) = chars.next() else {
        return 0.0;
    };
    let gid = charmap.map(first as u32).unwrap_or_default();
    metrics.advance_width(gid).unwrap_or(0.0)
}

/// Emit `GlyphPos` entries for every codepoint in `grapheme`.
///
/// The first codepoint carries the cluster advance; subsequent code
/// points (combining marks, ZWJ joiners) emit zero-advance positions
/// pointing at the same cluster byte. Skrifa returns the cmap's `notdef`
/// glyph (id 0) for unmapped codepoints — we keep them so downstream
/// can paint the tofu box.
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

/// Codepoint classification kept for downstream (G1.5 atlas routing).
///
/// G1.3 doesn't act on the result — skrifa returns the correct
/// advance for Latin, CJK, and emoji uniformly — but it pins the
/// detection contract so the followup atlas-routing dispatch has a
/// stable surface.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodepointClass {
    /// Standard Latin / Greek / Cyrillic etc.
    Regular,
    /// CJK Symbols and Punctuation (`U+3000..U+303F`).
    CjkPunctuation,
    /// Half/Full-width Forms (`U+FF00..U+FFEF`).
    HalfFullWidth,
    /// Emoji codepoint (BMP + supplementary blocks).
    Emoji,
}

/// Classify the lead codepoint of a grapheme.
#[cfg(test)]
pub(crate) fn classify(ch: char) -> CodepointClass {
    let cp = ch as u32;
    if matches!(cp, 0x3000..=0x303F) {
        CodepointClass::CjkPunctuation
    } else if matches!(cp, 0xFF00..=0xFFEF) {
        CodepointClass::HalfFullWidth
    } else if is_emoji(cp) {
        CodepointClass::Emoji
    } else {
        CodepointClass::Regular
    }
}

/// Cheap emoji-block check covering the high-frequency ranges.
///
/// Per DR-044: pulling `unicode-properties` for the well-typed
/// `EMOJI_PRESENTATION` query is overkill for G1.3 — the codepoint
/// ranges below cover Misc Symbols + Dingbats + the SMP emoji
/// blocks, which is what the test corpus needs. G1.5 can swap in
/// the full property table when the atlas-routing surface needs
/// stricter coverage.
#[cfg(test)]
const fn is_emoji(cp: u32) -> bool {
    matches!(
        cp,
        0x2600..=0x27BF      // Misc Symbols + Dingbats
        | 0x1F300..=0x1F5FF  // Misc Symbols and Pictographs
        | 0x1F600..=0x1F64F  // Emoticons
        | 0x1F680..=0x1F6FF  // Transport and Map
        | 0x1F700..=0x1F77F  // Alchemical
        | 0x1F900..=0x1F9FF, // Supplemental Symbols and Pictographs
    )
}

#[cfg(test)]
#[path = "shaper/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shaper/covers_tests.rs"]
mod covers_tests;
