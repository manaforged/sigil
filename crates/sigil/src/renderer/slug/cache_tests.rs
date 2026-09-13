#![allow(clippy::unwrap_used)]

use super::*;
use crate::renderer::slug::bands::{FLOATS_TOTAL, GlyphBbox};

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

    assert_eq!(cache.get(&key(10)).unwrap().slot, 0);
    assert_eq!(cache.get(&key(20)).unwrap().slot, 1);
    assert_eq!(cache.get(&key(30)).unwrap().slot, 2);

    cache.insert(key(20), band());
    assert_eq!(cache.get(&key(20)).unwrap().slot, 1);

    cache.insert(key(40), band());
    assert_eq!(cache.get(&key(40)).unwrap().slot, 3);
}
