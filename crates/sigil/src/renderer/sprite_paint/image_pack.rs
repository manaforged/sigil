use std::collections::HashMap;

use super::SpritePipeline;
use super::atlas::{AtlasDesc, MAX_ATLASES, build_atlas, valid_atlas_desc};

pub(super) const PAGE: u32 = 1024;
const PAD: u32 = 1;
const STALE_FRAMES: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PackedImage {
    pub atlas: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PackedImage {
    pub(crate) const NONE: Self = Self {
        atlas: u32::MAX,
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    pub(crate) const fn is_none(self) -> bool {
        self.atlas == u32::MAX
    }
}

struct Page {
    atlas: u32,
    x: u32,
    y: u32,
    shelf_h: u32,
}

#[derive(Default)]
pub(crate) struct ImagePack {
    pages: Vec<Page>,
    frame: u64,
    last_used: HashMap<u64, u64>,
    reset_pending: bool,
    free_dedicated: Vec<u32>,
}

impl ImagePack {
    fn owns_page(&self, atlas: u32) -> bool {
        self.pages.iter().any(|page| page.atlas == atlas)
    }

    fn evicts(&mut self, id: u64, packed: PackedImage, reset: bool) -> bool {
        let stale = self
            .last_used
            .get(&id)
            .is_none_or(|&used| used + STALE_FRAMES < self.frame);
        let on_page = self.owns_page(packed.atlas);
        let evict = stale || (reset && on_page);
        if evict {
            self.last_used.remove(&id);
            if !on_page {
                self.free_dedicated.push(packed.atlas);
            }
        }
        evict
    }

    fn reset_pages(&mut self) {
        for page in &mut self.pages {
            page.x = 0;
            page.y = 0;
            page.shelf_h = 0;
        }
    }

    pub(super) fn try_place(&mut self, w: u32, h: u32) -> Option<PackedImage> {
        let need_w = w.saturating_add(PAD);
        let need_h = h.saturating_add(PAD);
        if need_w > PAGE || need_h > PAGE {
            return None;
        }
        self.pages.iter_mut().find_map(|page| {
            page.try_place(need_w, need_h).map(|(x, y)| PackedImage {
                atlas: page.atlas,
                x,
                y,
                w,
                h,
            })
        })
    }

    pub(super) fn push_page(&mut self, atlas: u32, w: u32, h: u32) -> Option<PackedImage> {
        if self.pages.len() >= MAX_ATLASES {
            return None;
        }
        let need_w = w.saturating_add(PAD);
        let need_h = h.saturating_add(PAD);
        if need_w > PAGE || need_h > PAGE {
            return None;
        }
        let mut page = Page {
            atlas,
            x: 0,
            y: 0,
            shelf_h: 0,
        };
        let (x, y) = page.try_place(need_w, need_h)?;
        let packed = PackedImage { atlas, x, y, w, h };
        self.pages.push(page);
        Some(packed)
    }
}

impl SpritePipeline {
    pub(crate) fn begin_frame(&mut self) {
        let pack = &mut self.image_pack;
        pack.frame += 1;
        let reset = core::mem::take(&mut pack.reset_pending);
        self.image_slots
            .retain(|&id, &mut packed| !pack.evicts(id, packed, reset));
        if reset {
            pack.reset_pages();
        }
    }

    pub(crate) fn register_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &crate::kernel::ImageSource,
    ) -> PackedImage {
        let frame = self.image_pack.frame;
        if let Some(&packed) = self.image_slots.get(&source.id()) {
            self.image_pack.last_used.insert(source.id(), frame);
            return packed;
        }
        let packed = self.place_image(device, queue, source);
        if packed.is_none() {
            self.image_pack.reset_pending = true;
        } else {
            self.image_slots.insert(source.id(), packed);
            self.image_pack.last_used.insert(source.id(), frame);
        }
        packed
    }

    fn reuse_dedicated(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        desc: &AtlasDesc<'_>,
    ) -> Option<u32> {
        if !valid_atlas_desc(desc.bytes, desc.width, desc.height, desc.format) {
            return None;
        }
        let slot = self.image_pack.free_dedicated.pop()?;
        let index = slot as usize;
        let (entry, bind_group) =
            build_atlas(device, queue, &self.atlas_layout, &self.sampler, desc);
        *self.atlases.get_mut(index)? = entry;
        *self.atlas_bind_groups.get_mut(index)? = bind_group;
        Some(slot)
    }

    fn place_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &crate::kernel::ImageSource,
    ) -> PackedImage {
        let w = source.width();
        let h = source.height();
        if w == 0 || h == 0 {
            return PackedImage::NONE;
        }
        if w.saturating_add(1) > PAGE || h.saturating_add(1) > PAGE {
            return self.dedicated_image(device, queue, source);
        }
        if let Some(packed) = self.image_pack.try_place(w, h) {
            self.upload_packed(queue, packed, source.rgba());
            return packed;
        }
        let zeros = vec![0_u8; (PAGE * PAGE * 4) as usize];
        let atlas = self.register_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes: &zeros,
                width: PAGE,
                height: PAGE,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        );
        if atlas == u32::MAX {
            return PackedImage::NONE;
        }
        let Some(packed) = self.image_pack.push_page(atlas, w, h) else {
            return PackedImage::NONE;
        };
        self.upload_packed(queue, packed, source.rgba());
        packed
    }

    fn dedicated_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &crate::kernel::ImageSource,
    ) -> PackedImage {
        let w = source.width();
        let h = source.height();
        let desc = AtlasDesc {
            bytes: source.rgba(),
            width: w,
            height: h,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        };
        let atlas = self
            .reuse_dedicated(device, queue, &desc)
            .unwrap_or_else(|| self.register_atlas(device, queue, &desc));
        if atlas == u32::MAX {
            PackedImage::NONE
        } else {
            PackedImage {
                atlas,
                x: 0,
                y: 0,
                w,
                h,
            }
        }
    }

    fn upload_packed(&self, queue: &wgpu::Queue, packed: PackedImage, rgba: &[u8]) {
        let Some(entry) = self.atlases.get(packed.atlas as usize) else {
            return;
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &entry.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: packed.x,
                    y: packed.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(packed.w * 4),
                rows_per_image: Some(packed.h),
            },
            wgpu::Extent3d {
                width: packed.w,
                height: packed.h,
                depth_or_array_layers: 1,
            },
        );
    }
}

impl Page {
    const fn try_place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.shelf_h == 0 {
            if w > PAGE || h > PAGE {
                return None;
            }
            self.shelf_h = h;
            self.x = w;
            return Some((0, 0));
        }
        if h <= self.shelf_h && self.x.saturating_add(w) <= PAGE {
            let x = self.x;
            self.x += w;
            return Some((x, self.y));
        }
        let next_y = self.y.saturating_add(self.shelf_h);
        if next_y.saturating_add(h) > PAGE || w > PAGE {
            return None;
        }
        self.y = next_y;
        self.shelf_h = h;
        self.x = w;
        Some((0, next_y))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "image_pack_tests.rs"]
mod tests;
