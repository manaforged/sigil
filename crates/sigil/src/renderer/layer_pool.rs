use wgpu::TextureUsages;

const DIM_ALIGN: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerError {
    Empty,
    TooLarge { requested: u32, max: u32 },
}

pub struct LayerHandle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

pub struct LayerPool {
    free: Vec<LayerHandle>,
}

impl LayerPool {
    pub const MAX_ENTRIES: usize = 4;

    #[must_use]
    pub const fn new() -> Self {
        Self { free: Vec::new() }
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
            return Ok(self.free.swap_remove(index));
        }
        Ok(alloc_layer_texture(device, w, h, format))
    }

    pub fn release(&mut self, handle: LayerHandle) {
        if self.free.len() >= Self::MAX_ENTRIES {
            return;
        }
        self.free.push(handle);
    }

    pub fn clear(&mut self) {
        self.free.clear();
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.free.len()
    }

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
