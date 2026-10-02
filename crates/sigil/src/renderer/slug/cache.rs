use ahash::{AHashMap, AHashSet};

use super::bands::BandData;
use crate::hal::RenderedGlyph;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct GlyphKey {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub variation_coords: Box<[i16]>,
    pub skew_bits: u32,
}

impl GlyphKey {
    pub(super) fn of(glyph: &RenderedGlyph) -> Self {
        Self {
            font_id: glyph.font_id,
            font_index: glyph.font_index,
            glyph_id: glyph.glyph_id,
            variation_coords: glyph.variation_coords.clone(),
            skew_bits: glyph.skew.to_bits(),
        }
    }
}

pub(super) struct CachedGlyph {
    pub band_data: BandData,
    pub slot: u32,
}

pub(super) struct GlyphCache {
    map: AHashMap<GlyphKey, CachedGlyph>,
    empty: AHashSet<GlyphKey>,
    generation: u64,
}

impl GlyphCache {
    pub(super) fn new() -> Self {
        Self {
            map: AHashMap::new(),
            empty: AHashSet::new(),
            generation: 0,
        }
    }

    pub(super) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn get(&self, key: &GlyphKey) -> Option<&CachedGlyph> {
        self.map.get(key)
    }

    pub(super) fn contains(&self, key: &GlyphKey) -> bool {
        self.map.contains_key(key) || self.empty.contains(key)
    }

    pub(super) fn mark_empty(&mut self, key: GlyphKey) {
        self.empty.insert(key);
    }

    pub(super) fn insert(&mut self, key: GlyphKey, band_data: BandData) {
        let slot = self.map.get(&key).map_or_else(
            || u32::try_from(self.map.len()).unwrap_or(u32::MAX),
            |existing| existing.slot,
        );
        self.map.insert(key, CachedGlyph { band_data, slot });
        self.generation = self.generation.wrapping_add(1);
    }

    pub(super) fn len(&self) -> usize {
        self.map.len()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&GlyphKey, &CachedGlyph)> {
        self.map.iter()
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
