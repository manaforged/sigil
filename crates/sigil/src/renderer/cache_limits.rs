use crate::scene::command::SceneCommand;

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

struct Stamped<K, V> {
    key: K,
    commands: Vec<SceneCommand>,
    value: V,
    used: u64,
}

pub(crate) struct FrameCache<K, V> {
    entries: Vec<Stamped<K, V>>,
    frame: u64,
}

impl<K, V> Default for FrameCache<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            frame: 0,
        }
    }
}

impl<K: PartialEq, V> FrameCache<K, V> {
    pub(crate) fn begin_frame(&mut self, stale: u64) {
        self.frame += 1;
        let frame = self.frame;
        self.entries.retain(|entry| entry.used + stale >= frame);
    }

    pub(crate) fn get(&mut self, key: &K, commands: &[SceneCommand]) -> Option<&V> {
        let frame = self.frame;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.key == *key && entry.commands == commands)?;
        entry.used = frame;
        Some(&entry.value)
    }

    pub(crate) fn admit(
        &mut self,
        commands: &[SceneCommand],
        (need, limit): (u64, u64),
        area: impl Fn(&V) -> u64,
    ) -> bool {
        if commands.len() < CacheLimits::DEFAULT.layer_min_commands {
            return false;
        }
        let entries = &mut self.entries;
        let mut held: u64 = entries.iter().map(|entry| area(&entry.value)).sum();
        while held + need > limit {
            let Some(oldest) = entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.used < self.frame)
                .min_by_key(|(_, entry)| entry.used)
                .map(|(index, _)| index)
            else {
                return false;
            };
            held -= area(&entries.swap_remove(oldest).value);
        }
        true
    }

    pub(crate) fn insert(&mut self, key: K, commands: &[SceneCommand], value: V) {
        self.entries.push(Stamped {
            key,
            commands: commands.to_vec(),
            value,
            used: self.frame,
        });
    }
}
