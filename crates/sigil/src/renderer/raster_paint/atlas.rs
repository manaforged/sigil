use ahash::{AHashMap, AHashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::raster::{GlyphMask, RasterKey};

static TEST_INITIAL_SIZE: AtomicU32 = AtomicU32::new(0);
static TEST_MAX_SIZE: AtomicU32 = AtomicU32::new(0);

#[cfg(any(test, feature = "kittest"))]
pub fn set_test_atlas_size_override(initial: u32, max: u32) {
    TEST_INITIAL_SIZE.store(initial, Ordering::Relaxed);
    TEST_MAX_SIZE.store(max, Ordering::Relaxed);
}

#[cfg(any(test, feature = "kittest"))]
#[must_use]
pub(crate) fn test_atlas_size_override() -> (u32, u32) {
    (
        TEST_INITIAL_SIZE.load(Ordering::Relaxed),
        TEST_MAX_SIZE.load(Ordering::Relaxed),
    )
}

fn size_or(size: &AtomicU32, default: u32) -> u32 {
    match size.load(Ordering::Relaxed) {
        0 => default,
        v => v,
    }
}

#[derive(Clone, Copy)]
pub(super) struct Placement {
    pub(super) px: [f32; 4],
    pub(super) width: u32,
    pub(super) height: u32,
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
    data: Vec<u8>,
    last_used: u64,
}

static WARNED_ATLAS_FULL: AtomicBool = AtomicBool::new(false);

pub(super) struct RasterAtlas {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) size: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
    grown: bool,
    frame: u64,
    compact_pending: bool,
    cache: AHashMap<RasterKey, CachedMask>,
    pub(super) blank: AHashSet<RasterKey>,
}

impl RasterAtlas {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let size = size_or(&TEST_INITIAL_SIZE, CacheLimits::DEFAULT.atlas_initial);
        let (texture, view) = create_texture(device, size);
        Self {
            texture,
            view,
            size,
            cursor_x: 0,
            cursor_y: 0,
            shelf_height: 0,
            grown: false,
            frame: 0,
            compact_pending: false,
            cache: AHashMap::new(),
            blank: AHashSet::new(),
        }
    }

    pub(super) fn get(&mut self, key: &RasterKey) -> Option<Placement> {
        let cached = self.cache.get_mut(key)?;
        cached.last_used = self.frame;
        Some(Self::placement(cached))
    }

    pub(super) fn begin_frame(&mut self, queue: &wgpu::Queue) {
        self.frame += 1;
        if self.compact_pending {
            crate::profile::count(crate::profile::Counter::TextAtlasCompactions);
            self.compact_pending = false;
            self.evict_stale();
            self.repack(queue);
        }
    }

    fn evict_stale(&mut self) {
        let frame = self.frame;
        let before = self.cache.len();
        self.cache.retain(|_, cached| {
            cached.last_used + CacheLimits::DEFAULT.glyph_stale_frames >= frame
        });
        if self.cache.len() == before {
            self.cache.retain(|_, cached| cached.last_used + 1 >= frame);
        }
    }

    fn repack(&mut self, queue: &wgpu::Queue) {
        let mut entries: Vec<(RasterKey, CachedMask)> = self.cache.drain().collect();
        entries.sort_by_key(|(_, cached)| core::cmp::Reverse(cached.rect.h));
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.shelf_height = 0;
        for (key, mut cached) in entries {
            let Some(rect) = self.shelf_alloc(cached.rect.w, cached.rect.h) else {
                continue;
            };
            cached.rect = rect;
            upload(queue, &self.texture, rect, &cached.data);
            self.cache.insert(key, cached);
        }
    }

    pub(super) fn insert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: RasterKey,
        mask: GlyphMask,
    ) -> Option<Placement> {
        crate::profile::count(crate::profile::Counter::TextAtlasMisses);
        let rect = self.alloc_or_grow(device, queue, mask.width, mask.height)?;
        upload(queue, &self.texture, rect, &mask.data);
        let cached = CachedMask {
            rect,
            left: mask.left,
            top: mask.top,
            data: mask.data,
            last_used: self.frame,
        };
        let placement = Self::placement(&cached);
        self.cache.insert(key, cached);
        Some(placement)
    }

    #[allow(clippy::cast_precision_loss)]
    const fn placement(cached: &CachedMask) -> Placement {
        let r = cached.rect;
        Placement {
            px: [
                r.x as f32,
                r.y as f32,
                (r.x + r.w) as f32,
                (r.y + r.h) as f32,
            ],
            width: r.w,
            height: r.h,
            left: cached.left,
            top: cached.top,
        }
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
        if !self.grown {
            self.grow(device, queue);
            self.grown = true;
            if let Some(rect) = self.shelf_alloc(w, h) {
                return Some(rect);
            }
        }
        self.compact_pending = true;
        warn_atlas_full();
        None
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

    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let new_size = (self.size * 2).min(size_or(&TEST_MAX_SIZE, CacheLimits::DEFAULT.atlas_max));
        let (texture, view) = create_texture(device, new_size);
        for cached in self.cache.values() {
            upload(queue, &texture, cached.rect, &cached.data);
        }
        self.texture = texture;
        self.view = view;
        self.size = new_size;
    }
}

fn upload(queue: &wgpu::Queue, texture: &wgpu::Texture, rect: AtlasRect, data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: rect.x,
                y: rect.y,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(rect.w),
            rows_per_image: Some(rect.h),
        },
        wgpu::Extent3d {
            width: rect.w,
            height: rect.h,
            depth_or_array_layers: 1,
        },
    );
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
            "raster glyph atlas full; overflow glyphs use vector Slug until stale entries \
             are evicted at the next frame"
        );
    }
}
