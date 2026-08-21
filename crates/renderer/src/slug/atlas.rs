//! Band-buffer atlas: a single GPU storage buffer holding all per-glyph
//! `BandData` for the current frame, packed contiguously.
//!
//! The CPU `GlyphCache` remains the source of truth. [`GlyphAtlas::rebuild`]
//! is called every `draw_paragraph` but skips GPU re-uploads when the cache
//! generation has not changed since the last rebuild. The buffer grows by
//! doubling when needed and is never shrunk.

use std::collections::HashMap;

use super::{
    bands::FLOATS_TOTAL,
    cache::{GlyphCache, GlyphKey},
};

/// Number of `f32` entries per glyph in the atlas buffer.
const F32S_PER_GLYPH: u64 = FLOATS_TOTAL as u64;
/// Byte size of one glyph's `BandData` in the atlas buffer.
const BYTES_PER_GLYPH: u64 = F32S_PER_GLYPH * 4;

/// Initial atlas capacity: 8 MiB (~1 316 glyphs).
const INITIAL_ATLAS_BYTES: u64 = 8 * 1024 * 1024;

/// Single GPU storage buffer holding all `BandData` for a frame.
///
/// After [`GlyphAtlas::rebuild`] the offsets map records each glyph's
/// **f32 index** (i.e. `byte_offset / 4`) within the buffer, ready to
/// pass as an instance attribute into the shader.
pub(super) struct GlyphAtlas {
    /// The wgpu buffer (`STORAGE | COPY_DST`).
    buffer: wgpu::Buffer,
    /// Current allocated size in bytes.
    capacity: u64,
    /// Uploaded atlas layout, including per-glyph f32 offsets.
    layout: AtlasLayout,
}

/// CPU-side description of the atlas layout currently uploaded to the GPU.
struct AtlasLayout {
    /// Per-glyph f32 index into `buffer`, keyed by `GlyphKey`.
    offsets: HashMap<GlyphKey, u32>,
    /// Cache generation at the time of the last GPU upload.
    last_generation: u64,
}

/// Work needed for the current rebuild request.
enum RebuildDecision {
    /// Cache is empty; the layout has been cleared.
    Empty,
    /// Cache generation matches the uploaded layout; no upload or offset rebuild.
    Current,
    /// Cache changed; upload `required_bytes` into the GPU buffer.
    Upload { required_bytes: u64 },
}

impl AtlasLayout {
    /// Create an empty layout that forces the first non-empty upload.
    fn new() -> Self {
        Self {
            offsets: HashMap::new(),
            last_generation: u64::MAX,
        }
    }

    /// Returns the f32 index of `key` in the uploaded layout.
    fn offset(&self, key: &GlyphKey) -> Option<u32> {
        self.offsets.get(key).copied()
    }

    /// Decide whether this rebuild needs a GPU upload.
    fn prepare_rebuild(&mut self, glyph_count: u64, current_generation: u64) -> RebuildDecision {
        if glyph_count == 0 {
            self.offsets.clear();
            self.last_generation = current_generation;
            return RebuildDecision::Empty;
        }

        if current_generation == self.last_generation {
            return RebuildDecision::Current;
        }

        self.offsets.clear();
        RebuildDecision::Upload {
            required_bytes: glyph_count * BYTES_PER_GLYPH,
        }
    }

    /// Record one glyph's f32 offset in the layout being uploaded.
    fn record_offset(&mut self, key: &GlyphKey, byte_offset: u64) {
        #[allow(clippy::cast_possible_truncation)]
        self.offsets.insert(key.clone(), (byte_offset / 4) as u32);
    }

    /// Mark the current layout as uploaded for `generation`.
    const fn finish_upload(&mut self, generation: u64) {
        self.last_generation = generation;
    }
}

impl GlyphAtlas {
    /// Returns a reference to the underlying GPU buffer.
    pub(super) const fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Returns the f32 index of `key` in the atlas buffer, or `None` if not present.
    pub(super) fn get_offset(&self, key: &GlyphKey) -> Option<u32> {
        self.layout.offset(key)
    }

    /// Allocate the atlas with [`INITIAL_ATLAS_BYTES`] capacity.
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("slug-atlas"),
            size: INITIAL_ATLAS_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity: INITIAL_ATLAS_BYTES,
            layout: AtlasLayout::new(),
        }
    }

    /// Pack all glyphs in `cache` into the atlas buffer.
    ///
    /// Re-allocates the buffer only when the required size exceeds the current
    /// capacity (grows by doubling). GPU upload is skipped when the cache
    /// generation matches the last upload — the buffer is already current.
    pub(super) fn rebuild(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &GlyphCache,
    ) {
        let glyph_count = cache.len() as u64;
        let current_gen = cache.generation();
        let RebuildDecision::Upload { required_bytes } =
            self.layout.prepare_rebuild(glyph_count, current_gen)
        else {
            return;
        };

        // Grow the buffer by doubling if the current allocation is too small.
        if required_bytes > self.capacity {
            let mut new_cap = self.capacity;
            while new_cap < required_bytes {
                new_cap *= 2;
            }
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("slug-atlas"),
                size: new_cap,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = new_cap;
        }

        // Pack each glyph at its STABLE slot offset (`slot * BYTES_PER_GLYPH`),
        // not sequential iteration order. HashMap order is non-deterministic and
        // reshuffles as glyphs are inserted, so sequential packing reassigned a
        // glyph's offset on every re-upload — scrambling any draw already
        // recorded earlier in the same frame against the old offset (the
        // cold-cache "garbled first frame"). Slot addressing pins each offset for
        // the cache's lifetime, so mid-frame re-uploads never move a glyph.
        for (key, cached) in cache.iter() {
            let byte_offset = u64::from(cached.slot) * BYTES_PER_GLYPH;
            // Store the f32 index (byte_offset / 4) so the shader can index
            // directly into `array<f32>` without an extra shift.
            self.layout.record_offset(key, byte_offset);
            queue.write_buffer(&self.buffer, byte_offset, cached.band_data.as_bytes());
        }

        self.layout.finish_upload(current_gen);
    }
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;
