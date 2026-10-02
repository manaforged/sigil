use crate::profile::count;
use crate::style::Color;

use super::WgpuRenderer;
use crate::geometry::{Point, Rect};
#[cfg(not(target_arch = "wasm32"))]
use crate::profile::gpu::snapshot;
use crate::renderer::damage::{self, FrameKey, FullReason, Plan, Retained};
use crate::renderer::device_state::{DeviceState, ReadyState};
use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::renderer::split_encoder::{SplitEncoder, clear_value};
use crate::renderer::surface::WgpuSurface;

#[derive(Debug)]
pub struct ScreenSubmission<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub viewport: (f32, f32),
    pub scale: f32,
    pub clear: Option<Color>,
}

pub(super) fn acquire_frame(
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

pub(crate) fn record_clear(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    color: Color,
) {
    count(crate::profile::Counter::GpuRenderPasses);
    let _p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("sigil-renderer clear pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_value(color)),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

pub(crate) fn submit_clear(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: Color,
) {
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sigil-renderer clear encoder"),
    });
    record_clear(&mut enc, view, color);
    queue.submit(core::iter::once(enc.finish()));
}

#[allow(
    clippy::too_many_arguments,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn walk_screen(
    ready: &mut ReadyState,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut SplitEncoder,
    retained: &mut Vec<LayerHandle>,
    view: &wgpu::TextureView,
    viewport: (f32, f32),
    scale: f32,
    format: wgpu::TextureFormat,
    scene: &CommandScene,
    stats: &mut FrameStats,
) -> Result<(), RendererError> {
    ready.begin_frame();
    let ReadyState {
        scene_pipelines,
        fill_pipelines,
        shadow_pipelines,
        slug_pipelines,
        raster_pipelines,
        sprite_pipelines,
        decoration_pipelines,
        layer_pipelines,
        blur_pipelines,
        layer_pool,
        pending,
        ..
    } = ready;
    walk_command_scene(
        WalkCtx {
            device,
            queue,
            encoder,
            retained,
            view,
            viewport,
            scale,
            format,
            scene_pipelines,
            fill_pipelines,
            shadow_pipelines,
            slug_pipelines,
            raster_pipelines,
            sprite_pipelines,
            decoration_pipelines,
            layer_pipelines,
            blur_pipelines,
            layer_pool,
            pending,
            stats,
        },
        scene,
    )
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn surface_text_scale(surface: &WgpuSurface, viewport: (f32, f32)) -> f32 {
    surface.config.width as f32 / viewport.0.max(1.0)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "physical pixel counts are non-negative and far below u64::MAX"
)]
pub(super) fn physical_pixels(region: Rect, scale: f32) -> u64 {
    let width = (region.width * scale).ceil().max(0.0) as u64;
    let height = (region.height * scale).ceil().max(0.0) as u64;
    width * height
}

fn clear_screen(split: &mut SplitEncoder, target: &ScreenSubmission<'_>) {
    if let Some(color) = target.clear {
        split.clear(target.view, color);
    }
}

impl WgpuRenderer {
    pub fn submit_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
    ) -> Result<(), RendererError> {
        let encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer submit_screen encoder"),
            });
        if let Err(error) = self.prepare_screen(target) {
            target.queue.submit(core::iter::once(encoder.finish()));
            return Err(error);
        }
        let key = target.clear.map(|clear| FrameKey {
            target: target.view.texture().clone(),
            viewport: target.viewport,
            scale: target.scale,
            clear,
        });
        let plan = {
            let _plan = crate::profile::span(crate::profile::Phase::RenderPlan);
            self.screen_plan(key.as_ref(), scene)
        };
        if plan == Plan::Unchanged {
            target.queue.submit(core::iter::once(encoder.finish()));
            return Ok(());
        }
        let mut split = SplitEncoder::new(target.device, target.queue, encoder);
        if let DeviceState::Ready(ready) = &mut self.state
            && ready.device == *target.device
        {
            split.reuse_uploads(&mut ready.uploads);
            split.time_passes(&mut ready.gpu_timer);
        }
        let result = {
            let _encode = crate::profile::span(crate::profile::Phase::RenderEncode);
            self.draw_planned(target, scene, &plan, &mut split)
        };
        {
            let _retain = crate::profile::span(crate::profile::Phase::RenderRetain);
            self.retained = key.map(|key| Retained::new(key, scene.clone()));
        }
        {
            let _submit = crate::profile::span(crate::profile::Phase::RenderSubmit);
            self.finish_screen(target, split);
        }
        result
    }

    fn screen_plan(&self, key: Option<&FrameKey>, scene: &CommandScene) -> Plan {
        let plan = match (&self.retained, key) {
            (Some(retained), Some(key)) => retained.plan(key, scene),
            (None, Some(_)) => Plan::Full(FullReason::FirstFrame),
            (_, None) => Plan::Full(FullReason::Untracked),
        };
        damage::note_plan(&plan);
        plan
    }

    fn finish_screen(&mut self, target: &ScreenSubmission<'_>, split: SplitEncoder) {
        let (uploads, timer) = split.submit();
        if let DeviceState::Ready(ready) = &mut self.state
            && ready.device == *target.device
        {
            ready.uploads = Some(uploads);
            if timer.is_some() {
                ready.gpu_timer = timer;
            }
            #[cfg(not(target_arch = "wasm32"))]
            snapshot(&ready.instance);
        }
        self.last_frame_stats.record_profile();
    }

    pub fn drain_gpu_timings(&mut self) {
        if let DeviceState::Ready(ready) = &mut self.state
            && let Some(timer) = ready.gpu_timer.as_mut()
        {
            timer.drain(&ready.device);
        }
    }

    pub fn record_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RendererError> {
        self.prepare_screen(target)?;
        let real = std::mem::replace(
            encoder,
            target
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer record_screen placeholder"),
                }),
        );
        let mut split = SplitEncoder::new(target.device, target.queue, real);
        clear_screen(&mut split, target);
        let result = self.walk_screen_commands(target, scene, &mut split);
        *encoder = split.into_inner();
        #[cfg(not(target_arch = "wasm32"))]
        if self.device().is_some_and(|device| device == target.device) {
            snapshot(self.instance());
        }
        result
    }

    fn draw_planned(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
        plan: &Plan,
        split: &mut SplitEncoder,
    ) -> Result<(), RendererError> {
        let region = match (plan, target.clear) {
            (Plan::Regions(regions), Some(clear)) => Some((regions, clear)),
            _ => None,
        };
        let Some((region, clear)) = region else {
            clear_screen(split, target);
            let result = self.walk_screen_commands(target, scene, split);
            let full = Rect::new(Point::new(0.0, 0.0), target.viewport.0, target.viewport.1);
            self.last_frame_stats.redrawn_pixels = physical_pixels(full, target.scale);
            return result;
        };
        let damaged = damage::clipped(scene, region, clear);
        let result = self.walk_screen_commands(target, &damaged, split);
        self.last_frame_stats.redrawn_pixels = region
            .iter()
            .map(|rect| physical_pixels(*rect, target.scale))
            .sum();
        result
    }

    fn prepare_screen(&mut self, target: &ScreenSubmission<'_>) -> Result<(), RendererError> {
        self.last_frame_stats = FrameStats::default();
        if target.viewport.0 <= 0.0 || target.viewport.1 <= 0.0 {
            return Err(RendererError::InvalidViewport);
        }
        self.recycle_transients();
        Ok(())
    }

    fn walk_screen_commands(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
        encoder: &mut SplitEncoder,
    ) -> Result<(), RendererError> {
        let owns_device = self.device().is_some_and(|device| device == target.device);
        if owns_device {
            let ready = self.state.ready_mut()?;
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
            crate::renderer::offscreen::walk_with_temporary_pipelines(
                encoder,
                &mut self.frame_transients,
                crate::renderer::offscreen::OffscreenTarget::from(target),
                scene,
                &mut self.last_frame_stats,
            )
        }
    }

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
}
