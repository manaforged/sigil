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
    pub offset: u32,
}

#[derive(Default)]
pub(super) struct GlyphCache {
    pub(super) map: AHashMap<GlyphKey, CachedGlyph>,
    empty: AHashSet<GlyphKey>,
    pub(super) generation: u64,
    pub(super) floats: u32,
}

impl GlyphCache {
    pub(super) fn contains(&self, key: &GlyphKey) -> bool {
        self.map.contains_key(key) || self.empty.contains(key)
    }

    pub(super) fn mark_empty(&mut self, key: GlyphKey) {
        self.empty.insert(key);
    }

    pub(super) fn insert(&mut self, key: GlyphKey, band_data: BandData) {
        let len = band_data.data.len();
        let offset = match self.map.get(&key) {
            Some(existing) if existing.band_data.data.len() == len => existing.offset,
            _ => self.allocate(len),
        };
        self.map.insert(key, CachedGlyph { band_data, offset });
        self.generation = self.generation.wrapping_add(1);
    }

    fn allocate(&mut self, len: usize) -> u32 {
        let offset = self.floats;
        self.floats = offset.saturating_add(u32::try_from(len).unwrap_or(u32::MAX));
        offset
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
