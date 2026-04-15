//! Surface state owned by the renderer consumer.
//!
//! A [`WgpuSurface`] is returned from
//! [`WgpuRenderer::create_surface`][crate::WgpuRenderer] and then
//! threaded back into `resize` / `clear` calls. It owns the `'static`
//! `wgpu::Surface` (wgpu takes ownership of the window handle box when
//! the surface is created and keeps it alive internally for the
//! surface's lifetime) and the current `SurfaceConfiguration`.

/// Per-window GPU state: the wgpu surface and its current
/// `SurfaceConfiguration`.
///
/// Construct via
/// [`WgpuRenderer::create_surface`][crate::WgpuRenderer::create_surface].
/// Consumers should treat the struct as opaque — the public accessors
/// ([`Self::size`], [`Self::format`]) are for diagnostics and test
/// assertions only.
pub struct WgpuSurface {
    /// Live wgpu surface. `'static` because wgpu internally holds the
    /// `Box<dyn ...>` window handle that was passed to
    /// `create_surface`, so the OS window outlives the surface by
    /// construction (see `wgpu::Instance::create_surface` source).
    pub(crate) surface: wgpu::Surface<'static>,

    /// Current swapchain configuration. Mutated in place on resize.
    pub(crate) config: wgpu::SurfaceConfiguration,
}

impl WgpuSurface {
    /// Current surface dimensions in physical pixels.
    ///
    /// May lag the OS window size by one frame around resize events,
    /// since we only apply the new config when `resize` is called.
    #[must_use]
    pub const fn size(&self) -> ui_hal::PhysicalSize {
        ui_hal::PhysicalSize::new(self.config.width, self.config.height)
    }

    /// The chosen swapchain texture format.
    ///
    /// Exposed primarily for tests and for future pipeline objects
    /// that need to match the surface format.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}
