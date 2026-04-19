#![allow(clippy::redundant_pub_crate)]
//! Lazy wgpu device/adapter/queue state owned by [`WgpuRenderer`].
//!
//! Split out of [`crate::renderer`] to keep each file under the 400-line
//! hygiene cap and give scene-pipeline caching a home. See
//! [`crate::renderer`] for the public surface.
//!
//! [`WgpuRenderer`]: crate::WgpuRenderer

use crate::error::RendererError;
use crate::scene_paint::ScenePipeline;
use crate::slug::SlugTextPipeline;
use pollster::FutureExt as _;

/// Lazily-initialised GPU state.
///
/// `WgpuRenderer::new` only builds the `wgpu::Instance` — picking an
/// adapter before we have a presentable surface would let wgpu hand us
/// a GPU that later cannot be configured onto the user's window (hybrid
/// iGPU/dGPU laptops, remote-display protocols, WebGL-compat-only
/// fallbacks, …). The first call to `create_surface` promotes this to
/// `Ready` by picking an adapter that is known-compatible with that
/// concrete surface.
pub(crate) enum DeviceState {
    /// Instance-only state. No adapter/device has been requested yet.
    Uninitialized(wgpu::Instance),
    /// Adapter + device chosen against the first surface we created.
    Ready(Box<ReadyState>),
}

pub(crate) struct ReadyState {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) scene_pipelines: Vec<ScenePipeline>,
    pub(crate) slug_pipelines: Vec<SlugTextPipeline>,
}

impl ReadyState {
    pub(crate) fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
}

impl DeviceState {
    pub(crate) fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        Self::Uninitialized(instance)
    }

    pub(crate) fn instance(&self) -> &wgpu::Instance {
        match self {
            Self::Uninitialized(instance) => instance,
            Self::Ready(ready) => &ready.instance,
        }
    }

    pub(crate) fn ready(&self) -> Result<&ReadyState, RendererError> {
        match self {
            Self::Ready(ready) => Ok(&**ready),
            Self::Uninitialized(_) => Err(RendererError::NotInitialized),
        }
    }

    pub(crate) fn ready_mut(&mut self) -> Result<&mut ReadyState, RendererError> {
        match self {
            Self::Ready(ready) => Ok(&mut **ready),
            Self::Uninitialized(_) => Err(RendererError::NotInitialized),
        }
    }

    /// Promote `Uninitialized` → `Ready`. Idempotent after the first call.
    pub(crate) fn ensure_ready(
        &mut self,
        surface: &wgpu::Surface<'_>,
    ) -> Result<&ReadyState, RendererError> {
        if matches!(self, Self::Uninitialized(_)) {
            let placeholder =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let instance = match core::mem::replace(self, Self::Uninitialized(placeholder)) {
                Self::Uninitialized(i) => i,
                Self::Ready(_) => unreachable!(),
            };
            match request_adapter_and_device(&instance, surface) {
                Ok((adapter, device, queue)) => {
                    *self = Self::Ready(Box::new(ReadyState {
                        instance,
                        adapter,
                        device,
                        queue,
                        scene_pipelines: Vec::new(),
                        slug_pipelines: Vec::new(),
                    }));
                }
                Err(err) => {
                    *self = Self::Uninitialized(instance);
                    return Err(err);
                }
            }
        }
        match self {
            Self::Ready(ready) => Ok(&**ready),
            Self::Uninitialized(_) => unreachable!(),
        }
    }
}

/// Request an adapter compatible with `surface`, then a device from that adapter.
fn request_adapter_and_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RendererError> {
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
