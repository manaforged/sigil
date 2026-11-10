mod submit;

pub(crate) use submit::record_clear;

#[cfg(any(test, feature = "kittest"))]
mod screen;
#[cfg(any(test, feature = "kittest"))]
pub use screen::ScreenSubmission;
mod surface_frame;

use surface_frame::SurfaceFrame;

use crate::geometry::PhysicalSize;
use crate::hal::Renderer;
#[cfg(not(target_arch = "wasm32"))]
use crate::profile::gpu::snapshot;
use crate::style::Rgba;

#[cfg(feature = "desktop")]
use crate::renderer::device_state::ReadyState;
use crate::renderer::device_state::{DeviceState, OnLost};
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::surface::WgpuSurface;
use crate::renderer::window_handle::WindowHandle;
use crate::{Error, ErrorKind};

pub struct WgpuRenderer {
    pub(crate) state: DeviceState,
    frame_transients: Vec<crate::renderer::layer_pool::LayerHandle>,
    transparent: bool,
    pub(crate) last_frame_stats: FrameStats,
    retained: Option<crate::renderer::damage::Retained>,
    on_lost: Option<OnLost>,
    device_generation: u64,
}

impl core::fmt::Debug for WgpuRenderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WgpuRenderer")
            .field("transparent", &self.transparent)
            .field("last_frame_stats", &self.last_frame_stats)
            .field("device_generation", &self.device_generation)
            .finish_non_exhaustive()
    }
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl WgpuRenderer {
    pub fn new() -> Self {
        Self {
            state: DeviceState::new(),
            frame_transients: Vec::new(),
            transparent: false,
            last_frame_stats: FrameStats::default(),
            retained: None,
            on_lost: None,
            device_generation: 0,
        }
    }

    #[cfg(all(test, feature = "inspect"))]
    pub(crate) fn without_adapter() -> Self {
        let mut renderer = Self::new();
        renderer.state = DeviceState::without_backends();
        renderer
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(
        clippy::future_not_send,
        reason = "browser GPU futures resolve on the main-thread executor"
    )]
    pub(crate) fn request_device(
        &self,
        on_lost: crate::renderer::OnLost,
    ) -> impl core::future::Future<Output = Result<crate::renderer::DeviceParts, Error>> + 'static
    {
        self.state.request(on_lost)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn adopt_device(&mut self, parts: crate::renderer::DeviceParts) {
        self.drop_device_resources();
        self.state.adopt(parts);
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn device_lost(&self) -> bool {
        self.state.is_lost()
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub(crate) fn set_on_lost(&mut self, on_lost: OnLost) {
        self.on_lost = Some(on_lost);
    }

    fn drop_device_resources(&mut self) {
        self.frame_transients.clear();
        self.retained = None;
        self.device_generation += 1;
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn backend_status(&self) -> Option<crate::profile::BackendStatus> {
        self.state.ready().ok().map(ReadyState::backend_status)
    }

    pub const fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.transparent
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub const fn last_frame_stats(&self) -> FrameStats {
        self.last_frame_stats
    }

    #[must_use]
    #[cfg(any(test, feature = "kittest"))]
    pub fn instance(&self) -> &wgpu::Instance {
        self.state.instance()
    }

    #[cfg(any(test, feature = "inspect", feature = "kittest"))]
    #[must_use]
    pub fn device(&self) -> Option<&wgpu::Device> {
        self.state.ready().ok().map(|ready| &ready.device)
    }

    #[must_use]
    #[cfg(any(test, feature = "kittest"))]
    pub fn queue(&self) -> Option<&wgpu::Queue> {
        self.state.ready().ok().map(|ready| &ready.queue)
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn pipeline_cache_counts(&self) -> Option<(usize, usize, usize, usize, usize)> {
        self.state.ready().ok().map(|ready| {
            (
                ready.pipelines.scene_pipelines.len(),
                ready.pipelines.fill_pipelines.len(),
                ready.pipelines.shadow_pipelines.len(),
                ready.pipelines.slug_pipelines.len(),
                ready.pipelines.quad_pipelines.len(),
            )
        })
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn decoration_pipeline_cache_count(&self) -> Option<usize> {
        self.state
            .ready()
            .ok()
            .map(|ready| ready.pipelines.decoration_pipelines.len())
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn layer_pool_ready_for_tests(&self) -> bool {
        self.state
            .ready()
            .is_ok_and(|ready| ready.pipelines.layer_pool.is_some())
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn ensure_headless_for_tests(&mut self) -> Result<(), Error> {
        self.state.ensure_ready_headless().map(|_| ())
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn headless_device_for_tests() -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), Error>
    {
        DeviceState::request_headless()
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn attach_headless_for_tests(
        &mut self,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.state
            .adopt_headless(adapter.clone(), device.clone(), queue.clone());
    }

    pub fn prepare_compositing(&mut self, format: wgpu::TextureFormat) -> Result<(), Error> {
        let ready = self.state.ready_mut()?;
        LayerCompositePipeline::for_format(
            &mut ready.pipelines.layer_pipelines,
            &ready.device,
            format,
        );
        Ok(())
    }

    pub(crate) fn recover_lost_device(&mut self, surface: &mut WgpuSurface) -> Result<bool, Error> {
        let rebuilt = self.rebuild_lost_device(surface)?;
        self.adopt_surface(surface)?;
        Ok(rebuilt)
    }

    fn rebuild_lost_device(&mut self, surface: &WgpuSurface) -> Result<bool, Error> {
        if !self.state.is_lost() || cfg!(target_arch = "wasm32") {
            return Ok(false);
        }
        self.state.rebuild(&surface.surface, self.on_lost.clone())?;
        self.drop_device_resources();
        tracing::info!("GPU device rebuilt after loss");
        Ok(true)
    }

    fn adopt_surface(&mut self, surface: &mut WgpuSurface) -> Result<(), Error> {
        if surface.device_generation == self.device_generation {
            return Ok(());
        }
        surface.backbuffer = None;
        let ready = self.state.ready()?;
        surface.surface.configure(&ready.device, &surface.config);
        surface.device_generation = self.device_generation;
        self.prepare_compositing(surface.config.format)
    }

    pub fn render_command_scene(
        &mut self,
        surface: &mut WgpuSurface,
        scene: &CommandScene,
        clear_color: Rgba,
        viewport: (f32, f32),
    ) -> Result<(), Error> {
        self.recover_lost_device(surface)?;
        self.last_frame_stats = FrameStats::default();
        let Self {
            state,
            last_frame_stats,
            retained,
            ..
        } = self;
        let ready = state.ready_mut()?;
        let request = SurfaceFrame {
            scene,
            clear: clear_color,
            viewport,
        };
        let result = ready.render_surface(surface, &request, last_frame_stats, retained);
        #[cfg(not(target_arch = "wasm32"))]
        snapshot(&ready.instance);
        result
    }
}

impl Renderer for WgpuRenderer {
    type Error = Error;
    type Surface = WgpuSurface;
    type WindowHandle = WindowHandle;

    fn create_surface(
        &mut self,
        window: Self::WindowHandle,
        initial_size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error> {
        let handle = window.into_inner();
        let surface = self
            .state
            .instance()
            .create_surface(handle)
            .map_err(Error::gpu_surface)?;
        let ready = self.state.ensure_ready(&surface, self.on_lost.clone())?;
        let caps = surface.get_capabilities(&ready.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(Error::fixed(
                ErrorKind::Gpu,
                "GPU adapter reported no usable swapchain formats for this surface",
            ))?;
        let alpha_mode = select_alpha_mode(&caps.alpha_modes, self.transparent);
        let (width, height) = surface_extent(initial_size, ready.max_texture_dimension_2d());
        let config = wgpu::SurfaceConfiguration {
            color_space: wgpu::SurfaceColorSpace::Srgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | (caps.usages & wgpu::TextureUsages::COPY_DST),
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 1,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(&ready.device, &config);
        self.prepare_compositing(format)?;
        Ok(WgpuSurface {
            surface,
            config,
            backbuffer: None,
            device_generation: self.device_generation,
        })
    }

    fn resize(
        &mut self,
        surface: &mut Self::Surface,
        size: PhysicalSize,
    ) -> Result<(), Self::Error> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let ready = self.state.ready()?;
        let (width, height) = surface_extent(size, ready.max_texture_dimension_2d());
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&ready.device, &surface.config);
        Ok(())
    }
}

fn surface_extent(size: PhysicalSize, max_texture_dimension_2d: u32) -> (u32, u32) {
    let max_extent = if max_texture_dimension_2d == 0 {
        u32::MAX
    } else {
        max_texture_dimension_2d
    };
    (
        size.width.clamp(1, max_extent),
        size.height.clamp(1, max_extent),
    )
}

fn select_alpha_mode(
    modes: &[wgpu::CompositeAlphaMode],
    transparent: bool,
) -> wgpu::CompositeAlphaMode {
    use wgpu::CompositeAlphaMode as Mode;
    if transparent {
        for want in [Mode::PostMultiplied, Mode::PreMultiplied, Mode::Inherit] {
            if modes.contains(&want) {
                return want;
            }
        }
        tracing::warn!(
            "backend offers no alpha-composite mode ({modes:?}); overlay will be opaque"
        );
    }
    modes.first().copied().unwrap_or(Mode::Auto)
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
