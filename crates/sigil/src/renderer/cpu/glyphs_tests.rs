use super::GlyphCache;
use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::raster::RasterKey;

fn key(glyph_id: u32) -> RasterKey {
    RasterKey {
        font_id: 1,
        font_index: 0,
        glyph_id,
        physical_px_bits: 16.0_f32.to_bits(),
        bin: 0,
        coords: 0,
    }
}

#[test]
fn a_full_cache_drops_only_stale_glyphs() {
    let limits = CacheLimits::DEFAULT;
    let mut cache = GlyphCache::default();
    let stale = u32::try_from(limits.glyph_masks).unwrap_or(u32::MAX);
    for id in 0..stale {
        cache.masks.insert(key(id), (None, 0));
    }
    let hot_frame = limits.glyph_stale_frames + 10;
    cache.masks.insert(key(stale), (None, hot_frame));
    cache.frame = hot_frame;
    cache.begin_frame();
    assert_eq!(cache.masks.len(), 1);
    assert!(cache.masks.contains_key(&key(stale)));
}
