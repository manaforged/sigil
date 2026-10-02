#![allow(clippy::redundant_pub_crate)]

#[cfg(feature = "desktop")]
use crate::profile::{BackendKind, BackendStatus, FallbackCause};
use crate::renderer::blur_paint::BlurPipeline;
use crate::renderer::decoration_paint::DecorationPipeline;
use crate::renderer::error::RendererError;
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_upload::UploadBuffers;
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::raster_paint::RasterTextPipeline;
use crate::renderer::scene_command::walk::WalkPending;
use crate::renderer::scene_paint::ScenePipeline;
use crate::renderer::shadow_paint::ShadowPipeline;
use crate::renderer::slug::SlugTextPipeline;
use crate::renderer::split_encoder::GpuTimer;
use crate::renderer::sprite_paint::SpritePipeline;
#[cfg(not(target_arch = "wasm32"))]
use pollster::FutureExt as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) enum DeviceState {
    Uninitialized(wgpu::Instance),
    Ready(Box<ReadyState>),
}

pub(crate) struct ReadyState {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) scene_pipelines: Vec<ScenePipeline>,
    pub(crate) fill_pipelines: Vec<FillPipeline>,
    pub(crate) shadow_pipelines: Vec<ShadowPipeline>,
    pub(crate) slug_pipelines: Vec<SlugTextPipeline>,
    pub(crate) raster_pipelines: Vec<RasterTextPipeline>,
    pub(crate) sprite_pipelines: Vec<SpritePipeline>,
    pub(crate) decoration_pipelines: Vec<DecorationPipeline>,
    pub(crate) layer_pipelines: Vec<LayerCompositePipeline>,
    pub(crate) blur_pipelines: Vec<BlurPipeline>,
    pub(crate) layer_pool: Option<LayerPool>,
    pub(crate) pending: WalkPending,
    pub(crate) uploads: Option<UploadBuffers>,
    pub(crate) gpu_timer: Option<GpuTimer>,
    lost: Arc<AtomicBool>,
    #[cfg(feature = "desktop")]
    forced_fallback: bool,
}

impl ReadyState {
    fn boxed(instance: wgpu::Instance, parts: DeviceParts) -> Box<Self> {
        let (adapter, device, queue, lost, forced_fallback) = parts;
        #[cfg(not(feature = "desktop"))]
        let _ = forced_fallback;
        Box::new(Self {
            instance,
            adapter,
            device,
            queue,
            scene_pipelines: Vec::new(),
            fill_pipelines: Vec::new(),
            shadow_pipelines: Vec::new(),
            slug_pipelines: Vec::new(),
            raster_pipelines: Vec::new(),
            sprite_pipelines: Vec::new(),
            decoration_pipelines: Vec::new(),
            layer_pipelines: Vec::new(),
            blur_pipelines: Vec::new(),
            layer_pool: Some(LayerPool::new()),
            pending: WalkPending::default(),
            uploads: None,
            gpu_timer: None,
            lost,
            #[cfg(feature = "desktop")]
            forced_fallback,
        })
    }

    pub(crate) fn begin_frame(&mut self) {
        for pipeline in &mut self.raster_pipelines {
            pipeline.begin_frame(&self.queue);
        }
        for pipeline in &mut self.sprite_pipelines {
            pipeline.begin_frame();
        }
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn backend_status(&self) -> BackendStatus {
        let software = self.adapter.get_info().device_type == wgpu::DeviceType::Cpu;
        BackendStatus {
            kind: if software || self.forced_fallback {
                BackendKind::GpuSoftware
            } else {
                BackendKind::Gpu
            },
            fallback: if self.forced_fallback {
                FallbackCause::NoAdapter
            } else {
                FallbackCause::None
            },
        }
    }

    pub(crate) fn max_texture_dimension_2d(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }
}

impl DeviceState {
    pub(crate) fn new() -> Self {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        Self::Uninitialized(instance)
    }

    pub(crate) fn instance(&self) -> &wgpu::Instance {
        match self {
            Self::Uninitialized(instance) => instance,
            Self::Ready(ready) => &ready.instance,
        }
    }

    pub(crate) fn is_lost(&self) -> bool {
        match self {
            Self::Ready(ready) => ready.lost.load(Ordering::Acquire),
            Self::Uninitialized(_) => false,
        }
    }

    pub(crate) fn reset(&mut self) {
        if let Self::Ready(ready) = self {
            let instance = ready.instance.clone();
            *self = Self::Uninitialized(instance);
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

    pub(crate) fn ensure_ready(
        &mut self,
        surface: &wgpu::Surface<'_>,
    ) -> Result<&ReadyState, RendererError> {
        self.ensure_ready_for(Some(surface))?;
        match self {
            Self::Ready(ready) => Ok(&**ready),
            Self::Uninitialized(_) => unreachable!(),
        }
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn request_headless()
    -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RendererError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let (adapter, device, queue, _lost, _forced) = request_blocking(&instance, None)?;
        Ok((adapter, device, queue))
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn adopt_headless(
        &mut self,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) {
        let instance = self.instance().clone();
        let lost = watch_device(&device, None);
        *self = Self::Ready(ReadyState::boxed(
            instance,
            (adapter, device, queue, lost, false),
        ));
    }

    #[cfg(any(test, feature = "kittest"))]
    pub(crate) fn ensure_ready_headless(&mut self) -> Result<&mut ReadyState, RendererError> {
        self.ensure_ready_for(None)?;
        match self {
            Self::Ready(ready) => Ok(&mut **ready),
            Self::Uninitialized(_) => unreachable!(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(
        clippy::future_not_send,
        reason = "browser GPU futures resolve on the main-thread executor"
    )]
    pub(crate) fn request(
        &self,
        on_lost: OnLost,
    ) -> impl core::future::Future<Output = Result<DeviceParts, RendererError>> + 'static {
        let instance = self.instance().clone();
        async move { request_adapter_and_device(&instance, None, Some(on_lost)).await }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn adopt(&mut self, parts: DeviceParts) {
        let instance = self.instance().clone();
        *self = Self::Ready(ReadyState::boxed(instance, parts));
    }

    fn ensure_ready_for(
        &mut self,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<(), RendererError> {
        if matches!(self, Self::Uninitialized(_)) {
            let placeholder = wgpu::Instance::new(
                wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
            );
            let instance = match core::mem::replace(self, Self::Uninitialized(placeholder)) {
                Self::Uninitialized(i) => i,
                Self::Ready(_) => unreachable!(),
            };
            match request_blocking(&instance, surface) {
                Ok(parts) => *self = Self::Ready(ReadyState::boxed(instance, parts)),
                Err(err) => {
                    *self = Self::Uninitialized(instance);
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

pub(crate) type DeviceParts = (
    wgpu::Adapter,
    wgpu::Device,
    wgpu::Queue,
    Arc<AtomicBool>,
    bool,
);

pub(crate) type OnLost = Arc<dyn Fn() + Send + Sync>;

fn power_preference() -> wgpu::PowerPreference {
    wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::LowPower)
}

#[cfg(not(target_arch = "wasm32"))]
fn request_blocking(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<DeviceParts, RendererError> {
    request_adapter_and_device(instance, surface, None).block_on()
}

#[cfg(target_arch = "wasm32")]
const fn request_blocking(
    _instance: &wgpu::Instance,
    _surface: Option<&wgpu::Surface<'_>>,
) -> Result<DeviceParts, RendererError> {
    Err(RendererError::NotInitialized)
}

#[cfg_attr(
    target_arch = "wasm32",
    allow(
        clippy::future_not_send,
        reason = "browser GPU futures resolve on the main-thread executor"
    )
)]
async fn request_adapter_and_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    on_lost: Option<OnLost>,
) -> Result<DeviceParts, RendererError> {
    let (adapter, forced_fallback) = match request_adapter(instance, surface, false).await {
        Ok(adapter) => (adapter, false),
        Err(err) => {
            tracing::warn!("sigil-renderer: {err}; retrying with the software fallback adapter");
            let adapter = request_adapter(instance, surface, true)
                .await
                .map_err(|_| RendererError::NoAdapter)?;
            (adapter, true)
        }
    };

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("sigil-renderer device"),
            required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
            required_limits: renderer_limits(adapter.limits()),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await?;
    let lost = watch_device(&device, on_lost);
    Ok((adapter, device, queue, lost, forced_fallback))
}

#[cfg_attr(
    target_arch = "wasm32",
    allow(
        clippy::future_not_send,
        reason = "browser GPU futures resolve on the main-thread executor"
    )
)]
async fn request_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    force_fallback_adapter: bool,
) -> Result<wgpu::Adapter, wgpu::RequestAdapterError> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: power_preference(),
            compatible_surface: surface,
            force_fallback_adapter,
            apply_limit_buckets: false,
        })
        .await
}

fn watch_device(device: &wgpu::Device, on_lost: Option<OnLost>) -> Arc<AtomicBool> {
    let lost = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&lost);
    device.set_device_lost_callback(move |reason, message| {
        if reason == wgpu::DeviceLostReason::Destroyed {
            tracing::debug!(%message, "GPU device destroyed");
            return;
        }
        tracing::error!(%message, "GPU device lost; rebuilding on the next frame");
        flag.store(true, Ordering::Release);
        if let Some(on_lost) = &on_lost {
            on_lost();
        }
    });
    device.on_uncaptured_error(Arc::new(|error: wgpu::Error| {
        tracing::error!(%error, "uncaptured wgpu error");
    }));
    lost
}

const fn renderer_limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
    wgpu::Limits::downlevel_defaults().using_resolution(adapter_limits)
}

#[cfg(test)]
#[path = "device_state_tests.rs"]
mod tests;
