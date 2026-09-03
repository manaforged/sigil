#![allow(clippy::redundant_pub_crate)]

use crate::blur_paint::BlurPipeline;
use crate::decoration_paint::DecorationPipeline;
use crate::error::RendererError;
use crate::fill_paint::FillPipeline;
use crate::layer_composite::LayerCompositePipeline;
use crate::layer_pool::LayerPool;
use crate::raster_paint::RasterTextPipeline;
use crate::scene_paint::ScenePipeline;
use crate::shadow_paint::ShadowPipeline;
use crate::slug::SlugTextPipeline;
use crate::sprite_paint::SpritePipeline;
use pollster::FutureExt as _;

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
                        fill_pipelines: Vec::new(),
                        shadow_pipelines: Vec::new(),
                        slug_pipelines: Vec::new(),
                        raster_pipelines: Vec::new(),
                        sprite_pipelines: Vec::new(),
                        decoration_pipelines: Vec::new(),
                        layer_pipelines: Vec::new(),
                        blur_pipelines: Vec::new(),
                        layer_pool: Some(LayerPool::new()),
                    }));
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

fn request_adapter_and_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RendererError> {
    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: surface,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
    {
        Ok(adapter) => adapter,
        Err(_) => instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
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

    Ok((adapter, device, queue))
}

const fn renderer_limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
    wgpu::Limits::downlevel_defaults().using_resolution(adapter_limits)
}

#[cfg(test)]
#[path = "device_state_tests.rs"]
mod tests;
