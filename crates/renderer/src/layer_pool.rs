//! Offscreen texture pool for layer rendering.
//!
//! [`LayerPool::acquire`] reuses a free texture of the same rounded
//! `(width, height, format)` when one is available. [`LayerPool::release`]
//! returns a handle to that free list after the encoder that sampled it
//! has been submitted. Zero or oversized dimensions return
//! [`LayerError`] — callers must not skip the layer.

use wgpu::TextureUsages;

/// Minimum allocation granularity in pixels (both axes).
const DIM_ALIGN: u32 = 16;

/// Why [`LayerPool::acquire`] refused a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerError {
    /// Width or height was zero.
    Empty,
    /// Rounded size exceeds `max_texture_dimension_2d`.
    TooLarge {
        /// Requested rounded extent.
        requested: u32,
        /// Device limit.
        max: u32,
    },
}

/// Offscreen texture handle returned by [`LayerPool::acquire`].
///
/// Owns the texture. Return it with [`LayerPool::release`] after submit.
pub struct LayerHandle {
    /// The underlying wgpu texture.
    pub texture: wgpu::Texture,
    /// Default view over the full texture.
    pub view: wgpu::TextureView,
    /// Actual texture width (rounded up to 16 px).
    pub width: u32,
    /// Actual texture height (rounded up to 16 px).
    pub height: u32,
    /// Texture format.
    pub format: wgpu::TextureFormat,
}

/// Free-list of offscreen textures keyed by rounded size + format.
pub struct LayerPool {
    free: Vec<LayerHandle>,
}

impl LayerPool {
    /// Maximum free textures retained between frames.
    pub const MAX_ENTRIES: usize = 4;

    /// Create an empty pool.
    #[must_use]
    pub const fn new() -> Self {
        Self { free: Vec::new() }
    }

    /// Acquire a texture sized to `(width, height)` with `format`.
    ///
    /// Both dimensions are rounded up to the nearest 16-pixel multiple.
    ///
    /// # Errors
    ///
    /// [`LayerError::Empty`] when either dimension is zero.
    /// [`LayerError::TooLarge`] when the rounded size exceeds the device limit.
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

    /// Return a handle to the free list after its encoder has been submitted.
    pub fn release(&mut self, handle: LayerHandle) {
        if self.free.len() >= Self::MAX_ENTRIES {
            return;
        }
        self.free.push(handle);
    }

    /// Drop every free texture.
    pub fn clear(&mut self) {
        self.free.clear();
    }

    /// Number of free textures currently retained.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.free.len()
    }

    /// Returns `true` when the free list is empty.
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

/// Round `value` up to the nearest multiple of `align`.
///
/// `align` must be a non-zero power of two.
fn round_up(value: u32, align: u32) -> u32 {
    debug_assert!(align.is_power_of_two(), "align must be a power of two");
    value.wrapping_add(align - 1) & !(align - 1)
}

/// Allocate a new offscreen texture + default view.
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
