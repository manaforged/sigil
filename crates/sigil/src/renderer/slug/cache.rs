use std::collections::HashMap;

use super::bands::BandData;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct GlyphKey {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub variation_coords: Box<[i16]>,
}

pub(super) struct CachedGlyph {
    pub band_data: BandData,
    pub slot: u32,
}

pub(super) struct GlyphCache {
    map: HashMap<GlyphKey, CachedGlyph>,
    generation: u64,
}

impl GlyphCache {
    pub(super) fn new() -> Self {
        Self {
            map: HashMap::new(),
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
        self.map.contains_key(key)
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

    #[allow(dead_code, reason = "intentional, reviewed trade-off")]
    pub(super) fn clear(&mut self) {
        self.map.clear();
        self.generation = self.generation.wrapping_add(1);
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
