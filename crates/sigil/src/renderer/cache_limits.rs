#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CacheLimits {
    pub glyph_masks: usize,
    pub hinters: usize,
    pub glyph_stale_frames: u64,
    pub image_stale_frames: u64,
    pub atlas_initial: u32,
    pub atlas_max: u32,
    pub cpu_layer_pool: usize,
    pub cpu_layer_pixels: u64,
    pub cpu_layer_stale_frames: u64,
    pub layer_min_commands: usize,
    pub kept_layer_pixels: u64,
    pub kept_layer_stale_frames: u64,
    pub vector_pixels: u64,
    pub vector_stale_frames: u64,
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
        cpu_layer_pixels: 8 * 1024 * 1024,
        cpu_layer_stale_frames: 120,
        layer_min_commands: 8,
        kept_layer_pixels: 8 * 1024 * 1024,
        kept_layer_stale_frames: 120,
        vector_pixels: 8 * 1024 * 1024,
        vector_stale_frames: 120,
    };
}

pub(crate) fn make_room<E>(
    entries: &mut Vec<E>,
    frame: u64,
    (need, limit): (u64, u64),
    used: impl Fn(&E) -> u64,
    area: impl Fn(&E) -> u64,
) -> bool {
    let mut held: u64 = entries.iter().map(&area).sum();
    while held + need > limit {
        let Some(oldest) = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| used(entry) < frame)
            .min_by_key(|(_, entry)| used(entry))
            .map(|(index, _)| index)
        else {
            return false;
        };
        held -= area(&entries.swap_remove(oldest));
    }
    true
}
