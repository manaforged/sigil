mod submit;

pub(crate) use submit::record_clear;
pub use submit::{ScreenSubmission, WorldSubmission, submit_clear};

use submit::{acquire_frame, render_command_scene_surface, render_frame_surface};

use crate::hal::{Color, PhysicalSize, Renderer};
#[cfg(not(target_arch = "wasm32"))]
use crate::profile::gpu::snapshot;

use crate::renderer::device_state::DeviceState;
use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::surface::WgpuSurface;
use crate::renderer::window_handle::WindowHandle;

pub struct WgpuRenderer {
    pub(crate) state: DeviceState,
    frame_transients: Vec<crate::renderer::layer_pool::LayerHandle>,
    transparent: bool,
    pub(crate) last_frame_stats: FrameStats,
}

impl WgpuRenderer {
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            state: DeviceState::new(),
            frame_transients: Vec::new(),
            transparent: false,
            last_frame_stats: FrameStats::default(),
        })
    }

    pub const fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.transparent
    }

    #[must_use]
    pub const fn last_frame_stats(&self) -> FrameStats {
        self.last_frame_stats
    }

    #[must_use]
    pub fn instance(&self) -> &wgpu::Instance {
        self.state.instance()
    }

    #[must_use]
    pub fn device(&self) -> Option<&wgpu::Device> {
        self.state.ready().ok().map(|ready| &ready.device)
    }

    #[must_use]
    pub fn queue(&self) -> Option<&wgpu::Queue> {
        self.state.ready().ok().map(|ready| &ready.queue)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn pipeline_cache_counts(&self) -> Option<(usize, usize, usize, usize, usize)> {
        self.state.ready().ok().map(|ready| {
            (
                ready.scene_pipelines.len(),
                ready.fill_pipelines.len(),
                ready.shadow_pipelines.len(),
                ready.slug_pipelines.len(),
                ready.sprite_pipelines.len(),
            )
        })
    }

    #[must_use]
    pub fn decoration_pipeline_cache_count(&self) -> Option<usize> {
        self.state
            .ready()
            .ok()
            .map(|ready| ready.decoration_pipelines.len())
    }

    #[doc(hidden)]
    #[must_use]
    pub fn layer_pool_ready_for_tests(&self) -> bool {
        self.state
            .ready()
            .is_ok_and(|ready| ready.layer_pool.is_some())
    }

    #[doc(hidden)]
    pub fn ensure_headless_for_tests(&mut self) -> Result<(), RendererError> {
        self.state.ensure_ready_headless().map(|_| ())
    }

    pub fn prepare_compositing(
        &mut self,
        format: wgpu::TextureFormat,
    ) -> Result<(), RendererError> {
        let ready = self.state.ready_mut()?;
        LayerCompositePipeline::for_format(&mut ready.layer_pipelines, &ready.device, format);
        Ok(())
    }

    pub fn submit_world(
        &mut self,
        target: &WorldSubmission<'_>,
        _scene: &CommandScene,
    ) -> Result<(), RendererError> {
        if target.viewport.0 <= 0.0 || target.viewport.1 <= 0.0 {
            return Err(RendererError::InvalidViewport);
        }
        if !target.view_proj.iter().flatten().all(|f| f.is_finite()) {
            return Err(RendererError::InvalidViewProj);
        }
        Ok(())
    }

    pub fn render_frame<F>(
        &mut self,
        surface: &mut WgpuSurface,
        clear_color: Color,
        draw: F,
    ) -> Result<(), RendererError>
    where
        F: FnOnce(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView),
    {
        let ready = self.state.ready()?;
        render_frame_surface(ready, surface, clear_color, draw)
    }

    pub fn render_command_scene(
        &mut self,
        surface: &mut WgpuSurface,
        scene: &CommandScene,
        clear_color: Color,
        viewport: (f32, f32),
    ) -> Result<(), RendererError> {
        self.last_frame_stats = FrameStats::default();
        let Self {
            state,
            last_frame_stats,
            ..
        } = self;
        let ready = state.ready_mut()?;
        let result = render_command_scene_surface(
            ready,
            surface,
            scene,
            clear_color,
            viewport,
            last_frame_stats,
        );
        #[cfg(not(target_arch = "wasm32"))]
        snapshot(&ready.instance);
        result
    }
}

impl Renderer for WgpuRenderer {
    type Error = RendererError;
    type Surface = WgpuSurface;
    type WindowHandle = WindowHandle;

    fn create_surface(
        &mut self,
        window: Self::WindowHandle,
        initial_size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error> {
        let handle = window.into_inner();
        let surface = self.state.instance().create_surface(handle)?;
        let ready = self.state.ensure_ready(&surface)?;
        let caps = surface.get_capabilities(&ready.adapter);
        if caps.formats.is_empty() {
            return Err(RendererError::NoSupportedFormat);
        }
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(RendererError::NoSupportedFormat)?;
        let alpha_mode = select_alpha_mode(&caps.alpha_modes, self.transparent);
        let (width, height) = surface_extent(initial_size, ready.max_texture_dimension_2d());
        let config = wgpu::SurfaceConfiguration {
            color_space: wgpu::SurfaceColorSpace::Srgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
        Ok(WgpuSurface { surface, config })
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

    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error> {
        let ready = self.state.ready()?;
        let Some((frame, view)) = acquire_frame(ready, surface)? else {
            return Ok(());
        };
        submit_clear(&ready.device, &ready.queue, &view, color);
        ready.queue.present(frame);
        Ok(())
    }
}

const fn surface_extent(size: PhysicalSize, max_texture_dimension_2d: u32) -> (u32, u32) {
    let max_extent = if max_texture_dimension_2d == 0 {
        u32::MAX
    } else {
        max_texture_dimension_2d
    };
    (
        clamp_extent(size.width, max_extent),
        clamp_extent(size.height, max_extent),
    )
}

const fn clamp_extent(value: u32, max_extent: u32) -> u32 {
    if value == 0 {
        1
    } else if value > max_extent {
        max_extent
    } else {
        value
    }
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
