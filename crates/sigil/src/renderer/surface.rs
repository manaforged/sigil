pub struct WgpuSurface {
    pub(crate) surface: wgpu::Surface<'static>,

    pub(crate) config: wgpu::SurfaceConfiguration,
}

impl WgpuSurface {
    #[must_use]
    pub const fn size(&self) -> crate::hal::PhysicalSize {
        crate::hal::PhysicalSize::new(self.config.width, self.config.height)
    }

    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}
