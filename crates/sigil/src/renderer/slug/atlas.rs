use std::collections::HashMap;

use super::{
    bands::FLOATS_TOTAL,
    cache::{GlyphCache, GlyphKey},
};

const BYTES_PER_GLYPH: u64 = FLOATS_TOTAL as u64 * 4;

const INITIAL_ATLAS_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct GlyphAtlas {
    pub(super) buffer: wgpu::Buffer,
    capacity: u64,
    layout: AtlasLayout,
}

struct AtlasLayout {
    offsets: HashMap<GlyphKey, u32>,
    last_generation: u64,
}

enum RebuildDecision {
    Empty,
    Current,
    Upload { required_bytes: u64 },
}

impl AtlasLayout {
    fn new() -> Self {
        Self {
            offsets: HashMap::new(),
            last_generation: u64::MAX,
        }
    }

    fn prepare_rebuild(&mut self, glyph_count: u64, current_generation: u64) -> RebuildDecision {
        if glyph_count == 0 {
            self.offsets.clear();
            self.last_generation = current_generation;
            return RebuildDecision::Empty;
        }

        if current_generation == self.last_generation {
            return RebuildDecision::Current;
        }

        RebuildDecision::Upload {
            required_bytes: glyph_count * BYTES_PER_GLYPH,
        }
    }
}

impl GlyphAtlas {
    pub(super) fn get_offset(&self, key: &GlyphKey) -> Option<u32> {
        self.layout.offsets.get(key).copied()
    }

    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: storage_buffer(device, INITIAL_ATLAS_BYTES),
            capacity: INITIAL_ATLAS_BYTES,
            layout: AtlasLayout::new(),
        }
    }

    pub(super) fn rebuild(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &GlyphCache,
    ) {
        let glyph_count = cache.map.len() as u64;
        let current_gen = cache.generation;
        let RebuildDecision::Upload { required_bytes } =
            self.layout.prepare_rebuild(glyph_count, current_gen)
        else {
            return;
        };

        if required_bytes > self.capacity {
            let mut new_cap = self.capacity;
            while new_cap < required_bytes {
                new_cap *= 2;
            }
            self.buffer = storage_buffer(device, new_cap);
            self.capacity = new_cap;
            self.layout.offsets.clear();
        }

        for (key, cached) in &cache.map {
            if self.layout.offsets.contains_key(key) {
                continue;
            }
            let byte_offset = u64::from(cached.slot) * BYTES_PER_GLYPH;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "values are bounded to the target range before the cast"
            )]
            self.layout
                .offsets
                .insert(key.clone(), (byte_offset / 4) as u32);
            queue.write_buffer(&self.buffer, byte_offset, cached.band_data.as_bytes());
        }

        self.layout.last_generation = current_gen;
    }
}

fn storage_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("slug-atlas"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
