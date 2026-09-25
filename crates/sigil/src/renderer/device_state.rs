#![allow(clippy::redundant_pub_crate)]

use crate::renderer::blur_paint::BlurPipeline;
use crate::renderer::decoration_paint::DecorationPipeline;
use crate::renderer::error::RendererError;
use crate::renderer::fill_paint::FillPipeline;
use crate::renderer::frame_upload::UploadBuffers;
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::raster_paint::RasterTextPipeline;
use crate::renderer::scene_paint::ScenePipeline;
use crate::renderer::shadow_paint::ShadowPipeline;
use crate::renderer::slug::SlugTextPipeline;
use crate::renderer::sprite_paint::SpritePipeline;
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
    pub(crate) uploads: Option<UploadBuffers>,
    lost: Arc<AtomicBool>,
}

impl ReadyState {
    fn boxed(instance: wgpu::Instance, parts: DeviceParts) -> Box<Self> {
        let (adapter, device, queue, lost) = parts;
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
            uploads: None,
            lost,
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

    pub(crate) fn request_headless()
    -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RendererError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let (adapter, device, queue, _lost) = request_adapter_and_device(&instance, None)?;
        Ok((adapter, device, queue))
    }

    pub(crate) fn adopt_headless(
        &mut self,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) {
        let instance = self.instance().clone();
        let lost = watch_device(&device);
        *self = Self::Ready(ReadyState::boxed(instance, (adapter, device, queue, lost)));
    }

    pub(crate) fn ensure_ready_headless(&mut self) -> Result<&mut ReadyState, RendererError> {
        self.ensure_ready_for(None)?;
        match self {
            Self::Ready(ready) => Ok(&mut **ready),
            Self::Uninitialized(_) => unreachable!(),
        }
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
            match request_adapter_and_device(&instance, surface) {
                Ok((adapter, device, queue, lost)) => {
                    *self =
                        Self::Ready(ReadyState::boxed(instance, (adapter, device, queue, lost)));
                }
                Err(err) => {
                    *self = Self::Uninitialized(instance);
                    return Err(err);
                }
            }
        }
        Ok(())
    }
}

type DeviceParts = (wgpu::Adapter, wgpu::Device, wgpu::Queue, Arc<AtomicBool>);

fn power_preference() -> wgpu::PowerPreference {
    wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::LowPower)
}

fn request_adapter_and_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<DeviceParts, RendererError> {
    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: power_preference(),
            compatible_surface: surface,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
    {
        Ok(adapter) => adapter,
        Err(_) => instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_preference(),
                compatible_surface: surface,
                force_fallback_adapter: true,
                apply_limit_buckets: false,
            })
            .block_on()
            .map_err(|_| RendererError::NoAdapter)?,
    };

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("sigil-renderer device"),
            required_features: wgpu::Features::empty(),
            required_limits: renderer_limits(adapter.limits()),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()?;
    let lost = watch_device(&device);
    Ok((adapter, device, queue, lost))
}

fn watch_device(device: &wgpu::Device) -> Arc<AtomicBool> {
    let lost = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&lost);
    device.set_device_lost_callback(move |reason, message| {
        if reason == wgpu::DeviceLostReason::Destroyed {
            tracing::debug!(%message, "GPU device destroyed");
            return;
        }
        tracing::error!(%message, "GPU device lost; rebuilding on the next frame");
        flag.store(true, Ordering::Release);
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
