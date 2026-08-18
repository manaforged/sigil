//! Pack small raster images into shared atlas pages.
//!
//! One `ImageSource` used to consume one of [`MAX_ATLASES`] slots. Nav
//! path icons fill that registry and later images no-op. Pages hold many
//! small images; oversized images still take a dedicated slot.

use super::SpritePipeline;
use super::atlas::{AtlasDesc, MAX_ATLASES};

/// Side of one packed page, in texels.
pub(super) const PAGE: u32 = 1024;
/// Gap between packed images so linear sampling does not bleed.
const PAD: u32 = 1;

/// Placement of one image inside a registered atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PackedImage {
    /// Atlas id from [`super::SpritePipeline::register_atlas`].
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

/// Shelf cursor for the current packed page.
struct Page {
    atlas: u32,
    x: u32,
    y: u32,
    shelf_h: u32,
}

/// Shelf packer across one or more `PAGE`×`PAGE` atlases.
#[derive(Default)]
pub(crate) struct ImagePack {
    pages: Vec<Page>,
}

impl ImagePack {
    /// Place `w`×`h` on an existing page, or `None` if a new page is needed.
    pub(super) fn try_place(&mut self, w: u32, h: u32) -> Option<PackedImage> {
        let need_w = w.saturating_add(PAD);
        let need_h = h.saturating_add(PAD);
        if need_w > PAGE || need_h > PAGE {
            return None;
        }
        let page = self.pages.last_mut()?;
        page.try_place(need_w, need_h).map(|(x, y)| PackedImage {
            atlas: page.atlas,
            x,
            y,
            w,
            h,
        })
    }

    /// Start a new page on `atlas` and place the first image.
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
    /// Resolve a raster image to an atlas placement, uploading pixels
    /// once per content id. Small images share a page. Returns
    /// [`PackedImage::NONE`] when the registry is full or the buffer is
    /// invalid (the sprite draw then no-ops).
    pub(crate) fn register_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &sigil_kernel::ImageSource,
    ) -> PackedImage {
        if let Some(&packed) = self.image_slots.get(&source.id()) {
            return packed;
        }
        let packed = self.place_image(device, queue, source);
        if !packed.is_none() {
            self.image_slots.insert(source.id(), packed);
        }
        packed
    }

    fn place_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &sigil_kernel::ImageSource,
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
        source: &sigil_kernel::ImageSource,
    ) -> PackedImage {
        let w = source.width();
        let h = source.height();
        let atlas = self.register_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes: source.rgba(),
                width: w,
                height: h,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        );
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
        let row_bytes = packed.w as usize * 4;
        for row in 0..packed.h {
            let start = row as usize * row_bytes;
            let Some(row_data) = rgba.get(start..start + row_bytes) else {
                break;
            };
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &entry.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: packed.x,
                        y: packed.y + row,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                row_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32::try_from(row_bytes).unwrap_or(0)),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: packed.w,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn first_page_places_then_fills_the_shelf() {
        let mut pack = ImagePack::default();
        assert!(pack.try_place(16, 16).is_none());
        let a = pack.push_page(0, 16, 16).unwrap();
        assert_eq!(a.x, 0);
        assert_eq!(a.y, 0);
        let b = pack.try_place(16, 16).unwrap();
        assert_eq!(b.x, 17);
        assert_eq!(b.y, 0);
    }

    #[test]
    fn overflow_opens_a_new_shelf() {
        let mut pack = ImagePack::default();
        assert!(pack.push_page(0, PAGE - 1, 16).is_some());
        let next = pack.try_place(32, 16).unwrap();
        assert_eq!(next.x, 0);
        assert_eq!(next.y, 17);
    }
}
