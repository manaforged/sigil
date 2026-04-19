//! [`WgpuRenderer`] — the production [`sigil_hal::Renderer`] implementation.
//!
//! This module owns the public wgpu-facing API. Lazy device/adapter
//! construction lives in [`crate::device_state`]; the scene-paint
//! pipeline lives in [`crate::scene_paint`].

use sigil_hal::{Color, PhysicalSize, Renderer};

use crate::device_state::{DeviceState, ReadyState};
use crate::error::RendererError;
use crate::scene::Scene;
use crate::scene_command::CommandScene;
use crate::scene_command::adapter::flat_to_commands;
use crate::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::surface::WgpuSurface;
use crate::window_handle::WindowHandle;

/// wgpu-backed GPU renderer.
///
/// Owns a `wgpu::Instance` unconditionally and — after the first
/// `create_surface` call — an adapter, device, and queue chosen to be
/// compatible with the user's window. A single renderer can drive any
/// number of [`WgpuSurface`]s.
pub struct WgpuRenderer {
    state: DeviceState,
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
        })
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
        let Some((frame, view)) = acquire_frame(ready, surface)? else {
            return Ok(());
        };
        submit_clear(ready, &view, clear_color);
        draw(&ready.device, &ready.queue, &view);
        frame.present();
        Ok(())
    }

    /// Acquire the surface texture, clear, paint every `SceneItem::Rect`
    /// and `SceneItem::Text` from `scene`, then present.
    ///
    /// The scene's rectangles are interpreted in logical pixels; the
    /// `viewport` argument is also logical pixels and must match the
    /// layout root size used to emit the scene. The WGSL scene shader
    /// uses this logical viewport for its NDC conversion math. The
    /// acquired surface texture is the physical-pixel part of the call;
    /// do not pass physical surface dimensions as `viewport`.
    ///
    /// # Errors
    ///
    /// Returns [`RendererError::NotInitialized`] if called before
    /// [`Self::create_surface`]. Surface-lost / timeout returns `Ok(())` —
    /// the next frame retries.
    pub fn render_scene(
        &mut self,
        surface: &mut WgpuSurface,
        scene: &Scene,
        clear_color: Color,
        viewport: (f32, f32),
    ) -> Result<(), RendererError> {
        let commands = flat_to_commands(scene);
        self.render_command_scene(surface, &commands, clear_color, viewport)
    }

    /// Acquire the surface texture, clear, walk every command from `scene`,
    /// and present.
    ///
    /// This is the command-buffer counterpart to [`Self::render_scene`].
    /// `render_scene` remains the flat-scene compatibility entry point and
    /// compiles its input through the Step 3 adapter before calling this
    /// method.
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
        let format = surface.config.format;
        let ready = self.state.ready_mut()?;
        let Some((frame, view)) = acquire_frame(ready, surface)? else {
            return Ok(());
        };
        submit_clear(ready, &view, clear_color);

        // Split-borrow ReadyState so the walk can hold immutable refs to
        // device/queue while mutating the format-keyed pipeline caches.
        let ReadyState {
            device,
            queue,
            scene_pipelines,
            slug_pipelines,
            ..
        } = ready;
        walk_command_scene(
            WalkCtx {
                device,
                queue,
                view: &view,
                viewport,
                format,
                scene_pipelines,
                slug_pipelines,
            },
            scene,
        )?;
        frame.present();
        Ok(())
    }
}

/// Acquire the next surface texture. Returns `Ok(None)` when the surface
/// is lost / outdated / timing-out — caller should return cleanly.
fn acquire_frame(
    ready: &ReadyState,
    surface: &WgpuSurface,
) -> Result<Option<(wgpu::SurfaceTexture, wgpu::TextureView)>, RendererError> {
    let frame = match surface.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            surface.surface.configure(&ready.device, &surface.config);
            return Ok(None);
        }
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return Ok(None);
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            return Err(RendererError::SurfaceValidation);
        }
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    Ok(Some((frame, view)))
}

fn submit_clear(ready: &ReadyState, view: &wgpu::TextureView, color: Color) {
    let mut enc = ready
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sigil-renderer clear encoder"),
        });
    {
        let _p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(color.r),
                        g: f64::from(color.g),
                        b: f64::from(color.b),
                        a: f64::from(color.a),
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    ready.queue.submit(core::iter::once(enc.finish()));
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
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
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
        submit_clear(ready, &view, color);
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

#[cfg(test)]
mod tests {
    use super::{PhysicalSize, surface_extent};

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
}
