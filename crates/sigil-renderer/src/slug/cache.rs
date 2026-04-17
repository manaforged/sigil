//! Per-frame glyph band-data cache.

use std::collections::HashMap;

use super::bands::BandData;

/// Key uniquely identifying a glyph outline (resolution-independent).
///
/// Keyed on `(font_id, font_index, glyph_id, variation_coords)` so TTC faces
/// and variable-font instances each get their own cache entry.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct GlyphKey {
    /// Stable blob identity from `parley` / `skrifa`.
    pub font_id: u64,
    /// TrueType collection face index (0 for single-font files).
    pub font_index: u32,
    /// OpenType glyph ID.
    pub glyph_id: u32,
    /// Normalised variation coordinates as raw `i16` fixed-point bits.
    /// Empty slice for static (non-variable) fonts.
    pub variation_coords: Box<[i16]>,
}

/// A cache entry: preprocessed band data for one glyph.
pub(super) struct CachedGlyph {
    /// Slug band data ready for GPU upload.
    pub band_data: BandData,
}

/// Cache mapping [`GlyphKey`] → [`CachedGlyph`].
///
/// The cache is resolution-independent — one entry covers all font sizes.
/// It grows monotonically; call [`GlyphCache::clear`] once per scene if
/// needed (e.g. after a font reload).
///
/// The `generation` counter is incremented on every [`insert`](Self::insert)
/// so the atlas can detect when the cache content has changed and skip
/// redundant GPU re-uploads on stable frames.
pub(super) struct GlyphCache {
    map: HashMap<GlyphKey, CachedGlyph>,
    /// Monotonically increasing; bumped on every insertion.
    generation: u64,
}

impl GlyphCache {
    /// Create an empty cache.
    pub(super) fn new() -> Self {
        Self {
            map: HashMap::new(),
            generation: 0,
        }
    }

    /// Current generation counter.
    ///
    /// Two calls that return the same value guarantee no glyphs were inserted
    /// between them.
    pub(super) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Look up a cached glyph.
    pub(super) fn get(&self, key: &GlyphKey) -> Option<&CachedGlyph> {
        self.map.get(key)
    }

    /// Returns `true` if the glyph is already cached.
    pub(super) fn contains(&self, key: &GlyphKey) -> bool {
        self.map.contains_key(key)
    }

    /// Insert preprocessed band data for a glyph.
    pub(super) fn insert(&mut self, key: GlyphKey, band_data: BandData) {
        self.map.insert(key, CachedGlyph { band_data });
        self.generation = self.generation.wrapping_add(1);
    }

    /// Number of cached glyphs.
    pub(super) fn len(&self) -> usize {
        self.map.len()
    }

    /// Iterate over all `(GlyphKey, CachedGlyph)` pairs.
    pub(super) fn iter(&self) -> impl Iterator<Item = (&GlyphKey, &CachedGlyph)> {
        self.map.iter()
    }

    /// Evict all entries (e.g. after a font reload).
    #[allow(dead_code)]
    pub(super) fn clear(&mut self) {
        self.map.clear();
        self.generation = self.generation.wrapping_add(1);
    }
}
