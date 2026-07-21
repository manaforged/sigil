//! Shelf-packed R8 glyph atlas for the raster text path (DR-055 §1).
//!
//! One shared texture per [`super::RasterTextPipeline`], starting at 1024²
//! and growing once to 2048². Shelf packing is O(1) per glyph: pack
//! left-to-right along a shelf, start a new shelf when a glyph doesn't fit
//! the remaining width, grow when a shelf doesn't fit the remaining height.
//!
//! No eviction — a real application would need ~10k distinct small-glyph
//! masks to fill 2048² (DR-055's working-set math), far beyond a realistic
//! UI. When the atlas is full (already grown once), allocation fails and the
//! caller falls back to vector Slug for that glyph; a `tracing::warn!` fires
//! once per process so a real overflow is observable.
//! ponytail: no eviction — add LRU shelf reuse if a real app fills 2048².

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::raster::{GlyphMask, RasterKey};

const INITIAL_ATLAS_SIZE: u32 = 1024;
const MAX_ATLAS_SIZE: u32 = 2048;

// ---- Wave-3 D3 test hook (docs/plans/2026-07-07-text-quality-and-gap-closure.md) --
//
// Process-global override for the atlas's initial/max edge length, in
// texels. `0` (the default) disables the override and falls back to the
// real `INITIAL_ATLAS_SIZE`/`MAX_ATLAS_SIZE` constants above. Exists so an
// integration test can force a growth-boundary or atlas-full crossing with
// a paragraph of a few dozen glyphs instead of the ~10k a real 2048² atlas
// would need — see `tests/raster_atlas_growth.rs`.
static TEST_INITIAL_SIZE: AtomicU32 = AtomicU32::new(0);
static TEST_MAX_SIZE: AtomicU32 = AtomicU32::new(0);

/// Set (or clear, with `(0, 0)`) the test-only atlas size override. This is
/// a process global: callers must serialize (`--test-threads=1`) and reset
/// to `(0, 0)` after use (a `Drop` guard in the test is the safe pattern —
/// see `tests/raster_atlas_growth.rs`) so later tests see production sizes.
#[doc(hidden)]
pub fn set_test_atlas_size_override(initial: u32, max: u32) {
    TEST_INITIAL_SIZE.store(initial, Ordering::Relaxed);
    TEST_MAX_SIZE.store(max, Ordering::Relaxed);
}

/// Read the current test-only atlas size override.
///
/// Exists so [`crate::lock_global_text_state`] can snapshot and restore it;
/// prefer that guard over calling the setter directly.
#[must_use]
pub(crate) fn test_atlas_size_override() -> (u32, u32) {
    (
        TEST_INITIAL_SIZE.load(Ordering::Relaxed),
        TEST_MAX_SIZE.load(Ordering::Relaxed),
    )
}

fn initial_atlas_size() -> u32 {
    match TEST_INITIAL_SIZE.load(Ordering::Relaxed) {
        0 => INITIAL_ATLAS_SIZE,
        v => v,
    }
}

fn max_atlas_size() -> u32 {
    match TEST_MAX_SIZE.load(Ordering::Relaxed) {
        0 => MAX_ATLAS_SIZE,
        v => v,
    }
}

/// Resolved atlas placement for one cached glyph mask: texel-space rect
/// plus the pixel geometry the draw path needs to size and offset the quad.
#[derive(Clone, Copy)]
pub(super) struct Placement {
    /// `[x0, y0, x1, y1]` in atlas TEXELS, not normalised. A later glyph in
    /// the same accumulation can grow the atlas, which would invalidate any
    /// UV normalised against the pre-growth size — so normalisation happens
    /// once per draw, after all placements, against the final atlas size
    /// (see `place_glyphs`).
    pub(super) px: [f32; 4],
    /// Mask width/height in physical pixels (matches the atlas texel rect).
    pub(super) width: u32,
    pub(super) height: u32,
    /// Baseline-relative offsets, physical pixels (see `raster::GlyphMask`).
    pub(super) left: i32,
    pub(super) top: i32,
}

#[derive(Clone, Copy)]
struct AtlasRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

struct CachedMask {
    rect: AtlasRect,
    left: i32,
    top: i32,
    /// Retained so a grow can re-upload every cached glyph into the new,
    /// larger texture without re-rasterizing.
    data: Vec<u8>,
}

/// Session-wide latch: the atlas-full warning fires once per process, not
/// once per overflowing glyph.
static WARNED_ATLAS_FULL: AtomicBool = AtomicBool::new(false);

pub(super) struct RasterAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
    grown: bool,
    cache: HashMap<RasterKey, CachedMask>,
}

impl RasterAtlas {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let size = initial_atlas_size();
        let (texture, view) = create_texture(device, size);
        Self {
            texture,
            view,
            size,
            cursor_x: 0,
            cursor_y: 0,
            shelf_height: 0,
            grown: false,
            cache: HashMap::new(),
        }
    }

    pub(super) const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Look up an already-placed glyph without rasterizing.
    pub(super) fn get(&self, key: &RasterKey) -> Option<Placement> {
        self.cache.get(key).map(Self::placement)
    }

    /// Place a freshly rasterized `mask` in the atlas and upload it.
    /// Returns `None` when the atlas is full (already grown once) — the
    /// caller falls back to vector Slug for this glyph.
    pub(super) fn insert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: RasterKey,
        mask: GlyphMask,
    ) -> Option<Placement> {
        let rect = self.alloc_or_grow(device, queue, mask.width, mask.height)?;
        upload_rows(queue, &self.texture, rect, &mask.data);
        let cached = CachedMask {
            rect,
            left: mask.left,
            top: mask.top,
            data: mask.data,
        };
        let placement = Self::placement(&cached);
        self.cache.insert(key, cached);
        Some(placement)
    }

    const fn placement(cached: &CachedMask) -> Placement {
        let r = cached.rect;
        Placement {
            px: [
                f32_from_u32(r.x),
                f32_from_u32(r.y),
                f32_from_u32(r.x + r.w),
                f32_from_u32(r.y + r.h),
            ],
            width: r.w,
            height: r.h,
            left: cached.left,
            top: cached.top,
        }
    }

    /// Current atlas edge length in texels. Read AFTER all placements for a
    /// draw — growth during accumulation changes it.
    pub(super) const fn size(&self) -> u32 {
        self.size
    }

    fn alloc_or_grow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        w: u32,
        h: u32,
    ) -> Option<AtlasRect> {
        if let Some(rect) = self.shelf_alloc(w, h) {
            return Some(rect);
        }
        if self.grown {
            warn_atlas_full();
            return None;
        }
        self.grow(device, queue);
        self.grown = true;
        let rect = self.shelf_alloc(w, h);
        if rect.is_none() {
            warn_atlas_full();
        }
        rect
    }

    fn shelf_alloc(&mut self, w: u32, h: u32) -> Option<AtlasRect> {
        if w == 0 || h == 0 || w > self.size || h > self.size {
            return None;
        }
        if self.cursor_x + w > self.size {
            self.cursor_y += self.shelf_height;
            self.cursor_x = 0;
            self.shelf_height = 0;
        }
        if self.cursor_y + h > self.size {
            return None;
        }
        let rect = AtlasRect {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
        };
        self.cursor_x += w;
        self.shelf_height = self.shelf_height.max(h);
        Some(rect)
    }

    /// Recreate the texture at double size (capped at [`MAX_ATLAS_SIZE`], or
    /// the test override — see [`set_test_atlas_size_override`]) and
    /// re-upload every cached mask. Existing pixel rects stay valid — only
    /// the canvas grew — so shelf-packer cursor state is untouched.
    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let new_size = (self.size * 2).min(max_atlas_size());
        let (texture, view) = create_texture(device, new_size);
        for cached in self.cache.values() {
            upload_rows(queue, &texture, cached.rect, &cached.data);
        }
        self.texture = texture;
        self.view = view;
        self.size = new_size;
    }
}

#[allow(clippy::cast_precision_loss)]
const fn f32_from_u32(v: u32) -> f32 {
    v as f32
}

/// Upload `data` (tight-packed, `rect.w` bytes per row) one row at a time.
/// `write_texture` requires `bytes_per_row` to be a multiple of 256 for
/// multi-row copies; per-row (height = 1) copies are exempt, so this stays
/// correct for any glyph width without padding the CPU-side buffer.
fn upload_rows(queue: &wgpu::Queue, texture: &wgpu::Texture, rect: AtlasRect, data: &[u8]) {
    let row_bytes = rect.w as usize;
    for row in 0..rect.h {
        let start = row as usize * row_bytes;
        let Some(row_data) = data.get(start..start + row_bytes) else {
            break;
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x,
                    y: rect.y + row,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            row_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(rect.w),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: rect.w,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn create_texture(device: &wgpu::Device, size: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sigil-renderer raster glyph atlas"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn warn_atlas_full() {
    if WARNED_ATLAS_FULL
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            "raster glyph atlas full at 2048x2048 — falling back to vector Slug for \
             overflow glyphs; see DR-055 kill criteria for the LRU shelf-reuse upgrade"
        );
    }
}
