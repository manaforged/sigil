#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CacheLimits {
    pub glyph_masks: usize,
    pub hinters: usize,
    pub glyph_stale_frames: u64,
    pub image_stale_frames: u64,
    pub atlas_initial: u32,
    pub atlas_max: u32,
    pub cpu_layer_pool: usize,
}

impl CacheLimits {
    pub(crate) const DEFAULT: Self = Self {
        glyph_masks: 8_192,
        hinters: 48,
        glyph_stale_frames: 120,
        image_stale_frames: 300,
        atlas_initial: 1024,
        atlas_max: 2048,
        cpu_layer_pool: 8,
    };
}
