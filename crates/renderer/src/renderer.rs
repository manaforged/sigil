//! [`WgpuRenderer`] — the production [`sigil_hal::Renderer`] implementation.
//!
//! This module owns the public wgpu-facing API. Lazy device/adapter
//! construction lives in [`crate::device_state`]; the scene-paint
//! pipeline lives in [`crate::scene_paint`].

mod submit;

pub(crate) use submit::record_clear;
pub use submit::{ScreenSubmission, WorldSubmission, submit_clear};

use submit::{acquire_frame, render_command_scene_surface, render_frame_surface, walk_screen};

use sigil_hal::{Color, PhysicalSize, Renderer};

use crate::device_state::DeviceState;
use crate::error::RendererError;
use crate::frame_stats::FrameStats;
use crate::scene_command::CommandScene;
use crate::surface::WgpuSurface;
use crate::window_handle::WindowHandle;

/// wgpu-backed GPU renderer.
///
/// Owns a `wgpu::Instance` unconditionally and — after the first
/// `create_surface` call — an adapter, device, and queue chosen to be
/// compatible with the user's window. A single renderer can drive any
/// number of [`WgpuSurface`]s.
pub struct WgpuRenderer {
    pub(crate) state: DeviceState,
    /// Transient offscreen layer textures from the most recent `record_screen`.
    /// The host submits its encoder after `record_screen` returns, so these
    /// must outlive the call; they are cleared at the start of the next
    /// `record_screen` (by which point the prior submission has completed).
    frame_transients: Vec<crate::layer_pool::LayerHandle>,
    /// Request an alpha-composited surface from `create_surface`, so a
    /// transparent clear shows the desktop behind the window (overlays).
    transparent: bool,
    pub(crate) last_frame_stats: FrameStats,
}

impl WgpuRenderer {
    /// Construct a new renderer. Cheap — only creates a `wgpu::Instance`.
    /// Adapter and device selection are deferred to the first `create_surface`
    /// call so the GPU we pick is guaranteed compatible with the user's window.
    ///
    /// # Errors
    ///
    /// Currently infallible; returns `Result` for forward compatibility.
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            state: DeviceState::new(),
            frame_transients: Vec::new(),
            transparent: false,
            last_frame_stats: FrameStats::default(),
        })
    }

    /// Request (or drop) alpha-composited surfaces from subsequent
    /// [`Renderer::create_surface`] calls. Backends that offer no
    /// alpha-composite mode fall back to opaque with a warning.
    pub const fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    /// Whether alpha-composited surfaces have been requested.
    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.transparent
    }

    /// Exact command/compiler counters from the most recent scene render.
    #[must_use]
    pub const fn last_frame_stats(&self) -> FrameStats {
        self.last_frame_stats
    }

    /// Borrow the underlying `wgpu::Instance` (always available).
    #[must_use]
    pub fn instance(&self) -> &wgpu::Instance {
        self.state.instance()
    }

    /// Borrow the underlying `wgpu::Device`, or `None` before `create_surface`.
    #[must_use]
    pub fn device(&self) -> Option<&wgpu::Device> {
        self.state.ready().ok().map(|ready| &ready.device)
    }

    /// Borrow the underlying `wgpu::Queue`, or `None` before `create_surface`.
    #[must_use]
    pub fn queue(&self) -> Option<&wgpu::Queue> {
        self.state.ready().ok().map(|ready| &ready.queue)
    }

    /// Return `(scene, fill, shadow, slug, sprite)` pipeline-cache lengths.
    ///
    /// This is exposed for renderer integration tests that need to verify
    /// command walking has populated the format-keyed caches.
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

    /// Return the number of cached text-decoration pipelines, when initialized.
    #[must_use]
    pub fn decoration_pipeline_cache_count(&self) -> Option<usize> {
        self.state
            .ready()
            .ok()
            .map(|ready| ready.decoration_pipelines.len())
    }

    /// Return whether the renderer has initialized its offscreen layer pool.
    #[doc(hidden)]
    #[must_use]
    pub fn layer_pool_ready_for_tests(&self) -> bool {
        self.state
            .ready()
            .ok()
            .is_some_and(|ready| ready.layer_pool.is_some())
    }

    /// Initialize the renderer's headless device/queue without a surface.
    ///
    /// This is exposed for renderer integration tests that exercise offscreen
    /// command walking against the renderer-owned pipeline caches.
    #[doc(hidden)]
    pub fn ensure_headless_for_tests(&mut self) -> Result<(), RendererError> {
        self.state.ensure_ready_headless().map(|_| ())
    }

    /// Submit a screen-space scene into a caller-owned texture view.
    ///
    /// This is the documented public successor to the experimental
    /// [`render_command_scene_offscreen`][Self::render_command_scene_offscreen]
    /// API added in R11. The renderer does not acquire or present a swapchain
    /// surface — the caller owns the full frame graph.
    ///
    /// If `target.clear` is `Some(color)`, the view is cleared to that colour
    /// before drawing. Pass `None` when compositing on top of a pre-cleared
    /// target (e.g. the second `submit_screen` call in a split-path render).
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::InvalidViewport`] when either viewport
    /// dimension is zero or negative.
    pub fn submit_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
    ) -> Result<(), RendererError> {
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer submit_screen encoder"),
            });
        let result = self.record_screen(target, scene, &mut encoder);
        target.queue.submit(core::iter::once(encoder.finish()));
        result
    }

    /// Record a screen-space scene into a caller-owned `encoder` *without*
    /// submitting — the scene-wide analogue of
    /// [`SlugTextPipeline::record_paragraph`][crate::SlugTextPipeline::record_paragraph].
    ///
    /// The host submits `encoder`, letting it order this UI *after* its own
    /// passes (sprites / meshes) in a single submission so menus and HUD
    /// composite on top of the game frame. [`Self::submit_screen`] is the
    /// owns-the-encoder convenience wrapper around this. See DR-054.
    ///
    /// If `target.clear` is `Some(color)`, a clear pass is recorded first;
    /// otherwise the scene composites over existing content.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::InvalidViewport`] when either viewport
    /// dimension is zero or negative.
    pub fn record_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RendererError> {
        self.last_frame_stats = FrameStats::default();
        if target.viewport.0 <= 0.0 || target.viewport.1 <= 0.0 {
            return Err(RendererError::InvalidViewport);
        }
        if let Some(color) = target.clear {
            record_clear(encoder, target.view, color);
        }
        self.recycle_transients();
        let ready = self.state.ensure_ready_headless()?;
        // Device-equality guard: the renderer's cached pipelines are bound to
        // `ready.device`. Recording against a foreign device would poison the
        // caches with handles created on the wrong device. Foreign-device
        // callers get a fresh per-call pipeline set via
        // `walk_with_temporary_pipelines` (no cache mutation).
        if ready.device == *target.device {
            walk_screen(
                ready,
                target.device,
                target.queue,
                encoder,
                &mut self.frame_transients,
                target.view,
                target.viewport,
                target.scale,
                target.format,
                scene,
                &mut self.last_frame_stats,
            )
        } else {
            crate::offscreen::walk_with_temporary_pipelines(
                encoder,
                &mut self.frame_transients,
                crate::offscreen::OffscreenTarget::from(target),
                scene,
                &mut self.last_frame_stats,
            )
        }
    }

    /// Drop the previous call's transient layer textures (their submission
    /// has completed by now), so this call's captures reuse the same buffer.
    fn recycle_transients(&mut self) {
        if let DeviceState::Ready(ready) = &mut self.state {
            if let Some(pool) = ready.layer_pool.as_mut() {
                for handle in self.frame_transients.drain(..) {
                    pool.release(handle);
                }
            } else {
                self.frame_transients.clear();
            }
        } else {
            self.frame_transients.clear();
        }
    }

    /// Submit a world-space scene into a caller-owned texture view with depth
    /// occlusion.
    ///
    /// **Stub**: validates parameters and returns `Ok(())` without painting.
    /// the host engine's P13 phase lights up the real depth-aware implementation.
    /// The stub exists so the host engine can wire the entry point now and the
    /// depth-aware drawing fills in later.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::InvalidViewport`] when either viewport
    /// dimension is zero or negative.
    /// Returns [`RendererError::InvalidViewProj`] when the matrix contains a
    /// non-finite value (NaN or infinity).
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
        // the host engine's P13 fills in the real depth-aware rendering here.
        Ok(())
    }

    /// Acquire the surface texture, clear it to `clear_color`, run `draw`
    /// (which may composite additional GPU work such as text), then present.
    ///
    /// This is the integration point for [`crate::SlugTextPipeline`]: call
    /// `pipeline.draw_paragraph(...)` inside `draw` with the provided
    /// `device`, `queue`, and `view`. The frame is presented after `draw`
    /// returns, so all commands land in the same presentation.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::NotInitialized`] if called before
    /// [`Self::create_surface`]. Surface-lost / timeout returns `Ok(())` —
    /// the next frame retries.
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

    /// Acquire the surface texture, clear, walk every command from `scene`,
    /// and present.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::NotInitialized`] if called before
    /// [`Self::create_surface`]. Surface-lost / timeout returns `Ok(())` -
    /// the next frame retries.
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
        render_command_scene_surface(
            ready,
            surface,
            scene,
            clear_color,
            viewport,
            last_frame_stats,
        )
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(&ready.device, &config);
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
        frame.present();
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

/// Pick a surface composite-alpha mode. Opaque surfaces keep the
/// backend's first offering (previous behaviour); transparent surfaces
/// prefer the alpha-composited modes and warn when the backend offers
/// none, falling back to opaque rather than failing.
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
mod tests {
    use super::{PhysicalSize, select_alpha_mode, surface_extent};
    use wgpu::CompositeAlphaMode as Mode;

    #[test]
    fn surface_extent_clamps_to_adapter_limit() {
        assert_eq!(
            surface_extent(PhysicalSize::new(2864, 1524), 2048),
            (2048, 1524),
        );
    }

    #[test]
    fn surface_extent_never_returns_zero() {
        assert_eq!(surface_extent(PhysicalSize::new(0, 0), 2048), (1, 1));
    }

    #[test]
    fn transparent_prefers_post_multiplied() {
        // Typical Metal offering.
        let metal_modes = [Mode::Opaque, Mode::PostMultiplied];
        assert_eq!(select_alpha_mode(&metal_modes, true), Mode::PostMultiplied);
        // Vulkan-style premultiplied-only offering.
        let vulkan_modes = [Mode::Opaque, Mode::PreMultiplied];
        assert_eq!(select_alpha_mode(&vulkan_modes, true), Mode::PreMultiplied);
    }

    #[test]
    fn transparent_falls_back_to_opaque_when_unsupported() {
        assert_eq!(select_alpha_mode(&[Mode::Opaque], true), Mode::Opaque);
        assert_eq!(select_alpha_mode(&[], true), Mode::Auto);
    }

    #[test]
    fn opaque_path_keeps_first_offering() {
        let modes = [Mode::Opaque, Mode::PostMultiplied];
        assert_eq!(select_alpha_mode(&modes, false), Mode::Opaque);
        assert_eq!(select_alpha_mode(&[], false), Mode::Auto);
    }
}
