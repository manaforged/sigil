//! [`WgpuRenderer`] — the production [`sigil_hal::Renderer`] implementation.
//!
//! This module owns every touchpoint with `wgpu`. Consumers of
//! `sigil-renderer` should name [`WgpuRenderer`] only at construction time
//! and thereafter work through the [`sigil_hal::Renderer`] trait.
//! Composited rendering (clear + text in one frame) uses [`Self::render_frame`].

use sigil_hal::{Color, PhysicalSize, Renderer};

use crate::error::RendererError;
use crate::surface::WgpuSurface;
use crate::window_handle::WindowHandle;

/// Lazily-initialised GPU state.
///
/// `WgpuRenderer::new` only builds the `wgpu::Instance` — picking an
/// adapter before we have a presentable surface would let wgpu hand us
/// a GPU that later cannot be configured onto the user's window (hybrid
/// iGPU/dGPU laptops, remote-display protocols, WebGL-compat-only
/// fallbacks, …). The first call to `create_surface` promotes this to
/// `Ready` by picking an adapter that is known-compatible with that
/// concrete surface.
enum DeviceState {
    /// Instance-only state. No adapter/device has been requested yet.
    Uninitialized(wgpu::Instance),
    /// Adapter + device chosen against the first surface we created.
    Ready(ReadyState),
}

struct ReadyState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl ReadyState {
    fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
}

/// wgpu-backed GPU renderer.
///
/// Owns a `wgpu::Instance` unconditionally and — after the first
/// `create_surface` call — an adapter, device, and queue chosen to be
/// compatible with the user's window. A single renderer can drive any
/// number of [`WgpuSurface`]s.
pub struct WgpuRenderer {
    state: DeviceState,
}

/// Request an adapter compatible with `surface`, then a device from that adapter.
fn request_adapter_and_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RendererError> {
    use pollster::FutureExt as _;

    let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .block_on()
    else {
        return Err(RendererError::NoAdapter);
    };

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("sigil-renderer device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()?;

    Ok((adapter, device, queue))
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        Ok(Self {
            state: DeviceState::Uninitialized(instance),
        })
    }

    /// Promote `Uninitialized` → `Ready`. Idempotent after the first call.
    fn ensure_ready(&mut self, surface: &wgpu::Surface<'_>) -> Result<&ReadyState, RendererError> {
        if matches!(self.state, DeviceState::Uninitialized(_)) {
            let placeholder =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let instance = match core::mem::replace(
                &mut self.state,
                DeviceState::Uninitialized(placeholder),
            ) {
                DeviceState::Uninitialized(i) => i,
                DeviceState::Ready(_) => unreachable!(),
            };
            match request_adapter_and_device(&instance, surface) {
                Ok((adapter, device, queue)) => {
                    self.state = DeviceState::Ready(ReadyState {
                        instance,
                        adapter,
                        device,
                        queue,
                    });
                }
                Err(err) => {
                    self.state = DeviceState::Uninitialized(instance);
                    return Err(err);
                }
            }
        }
        match &self.state {
            DeviceState::Ready(ready) => Ok(ready),
            DeviceState::Uninitialized(_) => unreachable!(),
        }
    }

    /// Borrow `ReadyState`. Returns [`RendererError::NotInitialized`] before
    /// the first `create_surface` call.
    const fn ready(&self) -> Result<&ReadyState, RendererError> {
        match &self.state {
            DeviceState::Ready(ready) => Ok(ready),
            DeviceState::Uninitialized(_) => Err(RendererError::NotInitialized),
        }
    }

    /// Borrow the underlying `wgpu::Instance` (always available).
    #[must_use]
    pub const fn instance(&self) -> &wgpu::Instance {
        match &self.state {
            DeviceState::Uninitialized(instance) => instance,
            DeviceState::Ready(ready) => &ready.instance,
        }
    }

    /// Borrow the underlying `wgpu::Device`, or `None` before `create_surface`.
    #[must_use]
    pub const fn device(&self) -> Option<&wgpu::Device> {
        match &self.state {
            DeviceState::Ready(ready) => Some(&ready.device),
            DeviceState::Uninitialized(_) => None,
        }
    }

    /// Borrow the underlying `wgpu::Queue`, or `None` before `create_surface`.
    #[must_use]
    pub const fn queue(&self) -> Option<&wgpu::Queue> {
        match &self.state {
            DeviceState::Ready(ready) => Some(&ready.queue),
            DeviceState::Uninitialized(_) => None,
        }
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
        let ready = self.ready()?;
        let frame = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.surface.configure(&ready.device, &surface.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RendererError::SurfaceValidation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut enc = ready
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_frame clear"),
            });
        {
            let _p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_frame clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear_color.r),
                            g: f64::from(clear_color.g),
                            b: f64::from(clear_color.b),
                            a: f64::from(clear_color.a),
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

        draw(&ready.device, &ready.queue, &view);
        frame.present();
        Ok(())
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
        let surface = self.instance().create_surface(handle)?;
        let ready = self.ensure_ready(&surface)?;
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
        let ready = self.ready()?;
        let (width, height) = surface_extent(size, ready.max_texture_dimension_2d());
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&ready.device, &surface.config);
        Ok(())
    }

    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error> {
        let ready = self.ready()?;
        let frame = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.surface.configure(&ready.device, &surface.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RendererError::SurfaceValidation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = ready
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer clear encoder"),
            });
        {
            let _p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sigil-renderer clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
