#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
use wgpu::TextureUsages;

use crate::profile::{count, count_add, enabled, gauge};

const DIM_ALIGN: u32 = 16;
const BUDGET_SCREENS: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerError {
    Empty,
    TooLarge { requested: u32, max: u32 },
}

pub struct LayerHandle {
    #[cfg_attr(not(any(test, feature = "kittest")), allow(dead_code))]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl LayerHandle {
    fn bytes(&self) -> u64 {
        LayerPool::bytes((self.width, self.height), self.format)
    }
}

pub struct LayerPool {
    free: Vec<LayerHandle>,
    budget: Option<u64>,
}

impl LayerPool {
    pub const MAX_ENTRIES: usize = 4;

    #[must_use]
    pub const fn new() -> Self {
        Self {
            free: Vec::new(),
            budget: None,
        }
    }

    pub(crate) fn prepare(&mut self, texture: &wgpu::Texture) {
        let budget = Self::budget_for_target((texture.width(), texture.height()));
        self.budget = Some(budget);
        self.trim(budget);
        gauge(crate::profile::Counter::GpuLayerCacheBudgetBytes, budget);
        self.record_profile();
    }

    pub(crate) fn budget_for_target(extent: (u32, u32)) -> u64 {
        Self::bytes(extent, crate::renderer::scene_instance::LAYER_FORMAT) * BUDGET_SCREENS
    }

    pub(crate) fn bytes(extent: (u32, u32), format: wgpu::TextureFormat) -> u64 {
        let (block_width, block_height) = format.block_dimensions();
        let width = round_up(extent.0, DIM_ALIGN).div_ceil(block_width);
        let height = round_up(extent.1, DIM_ALIGN).div_ceil(block_height);
        u64::from(width) * u64::from(height) * u64::from(format.block_copy_size(None).unwrap_or(0))
    }

    pub fn acquire(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Result<LayerHandle, LayerError> {
        if width == 0 || height == 0 {
            return Err(LayerError::Empty);
        }
        let w = round_up(width, DIM_ALIGN);
        let h = round_up(height, DIM_ALIGN);
        let max_dim = device.limits().max_texture_dimension_2d;
        if w > max_dim || h > max_dim {
            return Err(LayerError::TooLarge {
                requested: w.max(h),
                max: max_dim,
            });
        }
        if let Some(index) = self
            .free
            .iter()
            .position(|slot| slot.width == w && slot.height == h && slot.format == format)
        {
            count(crate::profile::Counter::GpuLayerReuses);
            return Ok(self.free.remove(index));
        }
        count(crate::profile::Counter::AllocLayerTextures);
        if let Some(bytes) = format.block_copy_size(None) {
            count_add(
                crate::profile::Counter::AllocLayerBytes,
                u64::from(w) * u64::from(h) * u64::from(bytes),
            );
        }
        Ok(alloc_layer_texture(device, w, h, format))
    }

    pub fn release(&mut self, handle: LayerHandle) {
        if let Some(budget) = self.budget {
            let bytes = handle.bytes();
            if bytes > budget {
                return;
            }
            self.trim(budget - bytes);
        } else if self.free.len() >= Self::MAX_ENTRIES {
            drop(self.free.remove(0));
        }
        self.free.push(handle);
    }

    fn trim(&mut self, budget: u64) {
        let mut bytes = self.retained_bytes();
        while bytes > budget {
            let handle = self.free.remove(0);
            bytes -= handle.bytes();
        }
    }

    fn retained_bytes(&self) -> u64 {
        self.free.iter().map(LayerHandle::bytes).sum()
    }

    pub(crate) fn record_profile(&self) {
        if !enabled() {
            return;
        }
        gauge(
            crate::profile::Counter::GpuLayerCacheBytes,
            self.retained_bytes(),
        );
        gauge(
            crate::profile::Counter::GpuLayerCacheTextures,
            self.free.len() as u64,
        );
    }

    pub(crate) fn contains(&self, view: &wgpu::TextureView) -> bool {
        self.free.iter().any(|handle| handle.view == *view)
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn clear(&mut self) {
        self.free.clear();
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.free.len()
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.free.is_empty()
    }
}

impl Default for LayerPool {
    fn default() -> Self {
        Self::new()
    }
}

fn round_up(value: u32, align: u32) -> u32 {
    debug_assert!(align.is_power_of_two(), "align must be a power of two");
    value.wrapping_add(align - 1) & !(align - 1)
}

fn alloc_layer_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> LayerHandle {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sigil-renderer layer-pool texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    LayerHandle {
        texture,
        view,
        width,
        height,
        format,
    }
}

#[cfg(test)]
#[path = "layer_pool_tests.rs"]
mod tests;
