//! [`WgpuRenderer`] — the production [`ui_hal::Renderer`] implementation.
//!
//! This module owns every touchpoint with `wgpu`. Consumers of
//! `ui-renderer` should name [`WgpuRenderer`] only at construction time
//! and thereafter work through the [`ui_hal::Renderer`] trait.

use pollster::FutureExt as _;
use ui_hal::{Color, PhysicalSize, Renderer};

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
    /// Reused for any subsequent surfaces.
    Ready(ReadyState),
}

/// Fully-initialised GPU state. See [`DeviceState::Ready`].
struct ReadyState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// wgpu-backed GPU renderer.
///
/// Owns a `wgpu::Instance` unconditionally and — after the first
/// `create_surface` call — a `wgpu::Adapter`, `wgpu::Device`, and
/// `wgpu::Queue`. The adapter/device pair is chosen lazily against the
/// first surface so we never pick a GPU that cannot present to the
/// user's window.
///
/// A single renderer can drive any number of [`WgpuSurface`]s; every
/// mainstream desktop wgpu backend (Vulkan / DX12 / Metal) can present
/// to multiple surfaces from one device once the first adapter has been
/// picked with a compatible-surface hint.
pub struct WgpuRenderer {
    state: DeviceState,
}

impl WgpuRenderer {
    /// Construct a new renderer.
    ///
    /// Cheap — only creates a `wgpu::Instance`. Adapter and device
    /// selection are deferred to the first `create_surface` call so the
    /// GPU we pick is guaranteed to be able to present to the user's
    /// window (see [`DeviceState`] for the rationale).
    pub fn new() -> Result<Self, RendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        Ok(Self {
            state: DeviceState::Uninitialized(instance),
        })
    }

    /// Promote `Uninitialized` to `Ready` by picking an adapter
    /// compatible with `surface`, then requesting a device. Idempotent:
    /// if we are already `Ready`, just returns the cached state.
    ///
    /// `pollster::block_on` is fine here — see Phase 2 docs: the
    /// renderer's `create_surface` runs on the main (event-loop) thread
    /// which is specifically permitted to block on adapter/device
    /// initialisation.
    fn ensure_ready(&mut self, surface: &wgpu::Surface<'_>) -> Result<&ReadyState, RendererError> {
        if matches!(self.state, DeviceState::Uninitialized(_)) {
            // Take ownership of the existing instance so we can move it
            // into the `Ready` variant without cloning. We swap in a
            // cheap placeholder instance so that, if adapter/device
            // selection fails, the renderer is still in a valid
            // `Uninitialized` state for the caller to retry with a
            // different window.
            let placeholder =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let instance = match core::mem::replace(
                &mut self.state,
                DeviceState::Uninitialized(placeholder),
            ) {
                DeviceState::Uninitialized(instance) => instance,
                DeviceState::Ready(_) => unreachable!("matched Uninitialized above"),
            };

            let Ok(adapter) = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(surface),
                    force_fallback_adapter: false,
                })
                .block_on()
            else {
                // Put the real instance back so the next retry reuses
                // it — the placeholder is discarded.
                self.state = DeviceState::Uninitialized(instance);
                return Err(RendererError::NoAdapter);
            };

            let (device, queue) = match adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("ui-renderer device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    trace: wgpu::Trace::Off,
                })
                .block_on()
            {
                Ok(pair) => pair,
                Err(err) => {
                    self.state = DeviceState::Uninitialized(instance);
                    return Err(RendererError::from(err));
                }
            };

            self.state = DeviceState::Ready(ReadyState {
                instance,
                adapter,
                device,
                queue,
            });
        }

        match &self.state {
            DeviceState::Ready(ready) => Ok(ready),
            DeviceState::Uninitialized(_) => {
                unreachable!("promoted to Ready above")
            }
        }
    }

    /// Borrow the `ReadyState` if the renderer has been initialised.
    ///
    /// Used by `resize` / `clear` which only run after `create_surface`
    /// has already run (by trait contract: a surface cannot exist
    /// without this side effect), so returning `None` here is a bug in
    /// the caller — we surface it as [`RendererError::NotInitialized`].
    const fn ready(&self) -> Result<&ReadyState, RendererError> {
        match &self.state {
            DeviceState::Ready(ready) => Ok(ready),
            DeviceState::Uninitialized(_) => Err(RendererError::NotInitialized),
        }
    }

    /// Borrow the underlying `wgpu::Instance`.
    ///
    /// Always available — created at `new`. Exposed for tests.
    #[must_use]
    pub const fn instance(&self) -> &wgpu::Instance {
        match &self.state {
            DeviceState::Uninitialized(instance) => instance,
            DeviceState::Ready(ready) => &ready.instance,
        }
    }

    /// Borrow the underlying `wgpu::Device`, if the renderer has
    /// selected one. Returns `None` before the first `create_surface`
    /// call.
    ///
    /// Exposed for tests and for future pipeline objects inside this
    /// crate. Downstream consumers should stay on the
    /// [`ui_hal::Renderer`] trait.
    #[must_use]
    pub const fn device(&self) -> Option<&wgpu::Device> {
        match &self.state {
            DeviceState::Ready(ready) => Some(&ready.device),
            DeviceState::Uninitialized(_) => None,
        }
    }

    /// Borrow the underlying `wgpu::Queue`, if the renderer has
    /// selected one. Returns `None` before the first `create_surface`
    /// call.
    #[must_use]
    pub const fn queue(&self) -> Option<&wgpu::Queue> {
        match &self.state {
            DeviceState::Ready(ready) => Some(&ready.queue),
            DeviceState::Uninitialized(_) => None,
        }
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

        // Create the surface first — we need it to pick a compatible
        // adapter. `wgpu::Instance::create_surface` takes
        // `impl Into<SurfaceTarget>`. Our `Box<dyn DisplayAndWindow>`
        // satisfies the blanket `From<T: DisplayAndWindowHandle + 'a>
        // for SurfaceTarget<'a>` because the trait object impls
        // `HasWindowHandle + HasDisplayHandle + Send + Sync` (see
        // `window_handle.rs`).
        let surface = self.instance().create_surface(handle)?;

        // Pick an adapter known to support this surface, then request a
        // device. Idempotent after the first call.
        let ready = self.ensure_ready(&surface)?;

        let caps = surface.get_capabilities(&ready.adapter);

        if caps.formats.is_empty() {
            return Err(RendererError::NoSupportedFormat);
        }

        // Prefer an sRGB format so colour values in `Color` — which the
        // HAL documents as linear — round-trip correctly through the
        // swapchain. Fall back to the first format the adapter offers.
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

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            // Clamp to 1 — wgpu rejects zero-sized swapchains, and a
            // freshly-created window can report `(0, 0)` before its
            // first `Resized` event on some platforms.
            width: initial_size.width.max(1),
            height: initial_size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };

        surface.configure(&ready.device, &config);

        // wgpu has internally taken ownership of the boxed handle (see
        // `wgpu::Instance::create_surface`, which stashes it onto
        // `_handle_source`). It outlives the surface for free — no
        // need for us to re-hold it.
        Ok(WgpuSurface { surface, config })
    }

    fn resize(
        &mut self,
        surface: &mut Self::Surface,
        size: PhysicalSize,
    ) -> Result<(), Self::Error> {
        // Trait contract: zero-sized resize is a no-op. This matches
        // how `winit` reports minimise events on Windows and Wayland.
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let ready = self.ready()?;
        surface.config.width = size.width;
        surface.config.height = size.height;
        surface.surface.configure(&ready.device, &surface.config);
        Ok(())
    }

    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error> {
        let ready = self.ready()?;

        let frame = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,

            // Routine during window resize / monitor reconfigure —
            // reconfigure and drop this frame, the next redraw request
            // will retry. Not an error to surface to callers.
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.surface.configure(&ready.device, &surface.config);
                return Ok(());
            }

            // Transient — the driver will recover on a later redraw.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }

            // Actual bug — surface up so the consumer can tear down
            // rather than spinning forever on a broken surface.
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RendererError::SurfaceValidation);
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = ready
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ui-renderer clear encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-renderer clear pass"),
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

        ready.queue.submit(core::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
