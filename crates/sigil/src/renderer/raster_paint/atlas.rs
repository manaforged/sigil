use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::renderer::raster::{GlyphMask, RasterKey};

const INITIAL_ATLAS_SIZE: u32 = 1024;
const MAX_ATLAS_SIZE: u32 = 2048;

static TEST_INITIAL_SIZE: AtomicU32 = AtomicU32::new(0);
static TEST_MAX_SIZE: AtomicU32 = AtomicU32::new(0);

#[doc(hidden)]
pub fn set_test_atlas_size_override(initial: u32, max: u32) {
    TEST_INITIAL_SIZE.store(initial, Ordering::Relaxed);
    TEST_MAX_SIZE.store(max, Ordering::Relaxed);
}

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
}

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

    pub(super) fn get(&self, key: &RasterKey) -> Option<Placement> {
        self.cache.get(key).map(Self::placement)
    }

    pub(super) fn insert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: RasterKey,
        mask: GlyphMask,
    ) -> Option<Placement> {
        let rect = self.alloc_or_grow(device, queue, mask.width, mask.height)?;
        upload(queue, &self.texture, rect, &mask.data);
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

    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let new_size = (self.size * 2).min(max_atlas_size());
        let (texture, view) = create_texture(device, new_size);
        for cached in self.cache.values() {
            upload(queue, &texture, cached.rect, &cached.data);
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
            "raster glyph atlas full at 2048x2048 — falling back to vector Slug for \
             overflow glyphs; see DR-055 kill criteria for the LRU shelf-reuse upgrade"
        );
    }
}
