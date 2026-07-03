//! Per-frame glyph band-data cache.

use std::collections::HashMap;

use super::bands::BandData;

/// Key uniquely identifying a glyph outline (resolution-independent).
///
/// Keyed on `(font_id, font_index, glyph_id, variation_coords)` so TTC faces
/// and variable-font instances each get their own cache entry.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct GlyphKey {
    /// Stable blob identity — pointer hash of the font's `Arc<Vec<u8>>` buffer
    /// as carried through the owned text engine + skrifa font ref.
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
    /// Stable atlas slot assigned at first insertion. The glyph's byte offset
    /// in the atlas buffer is `slot * BYTES_PER_GLYPH`, so the offset is fixed
    /// for the cache's lifetime regardless of `HashMap` iteration order. This is
    /// what keeps a draw's baked offset valid after a later glyph forces a
    /// mid-frame atlas re-upload (see `GlyphAtlas::rebuild`).
    pub slot: u32,
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
    ///
    /// A new key is assigned the next dense slot (`map.len()`); re-inserting an
    /// existing key keeps its slot so its atlas offset never moves. Slots stay
    /// dense and stable for the cache's lifetime, which is what the atlas relies
    /// on to keep already-baked draw offsets valid across re-uploads.
    pub(super) fn insert(&mut self, key: GlyphKey, band_data: BandData) {
        let slot = self.map.get(&key).map_or_else(
            || u32::try_from(self.map.len()).unwrap_or(u32::MAX),
            |existing| existing.slot,
        );
        self.map.insert(key, CachedGlyph { band_data, slot });
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::slug::bands::{FLOATS_TOTAL, GlyphBbox};

    fn band() -> BandData {
        BandData {
            data: vec![0.0; FLOATS_TOTAL],
            bbox: GlyphBbox {
                x_min: 0.0,
                y_min: 0.0,
                x_max: 1.0,
                y_max: 1.0,
            },
            units_per_em: 1000,
        }
    }

    fn key(glyph_id: u32) -> GlyphKey {
        GlyphKey {
            font_id: 1,
            font_index: 0,
            glyph_id,
            variation_coords: Box::new([]),
        }
    }

    #[test]
    fn slots_are_dense_stable_and_survive_reinsert() {
        let mut cache = GlyphCache::new();
        cache.insert(key(10), band());
        cache.insert(key(20), band());
        cache.insert(key(30), band());

        // Fresh keys take dense, insertion-ordered slots.
        assert_eq!(cache.get(&key(10)).unwrap().slot, 0);
        assert_eq!(cache.get(&key(20)).unwrap().slot, 1);
        assert_eq!(cache.get(&key(30)).unwrap().slot, 2);

        // Re-inserting an existing key MUST keep its slot — the atlas derives a
        // glyph's byte offset from its slot, and a mid-frame offset move is the
        // cold-cache scramble this addressing scheme exists to prevent.
        cache.insert(key(20), band());
        assert_eq!(cache.get(&key(20)).unwrap().slot, 1);

        // A brand-new glyph after the churn still appends densely.
        cache.insert(key(40), band());
        assert_eq!(cache.get(&key(40)).unwrap().slot, 3);
    }
}
