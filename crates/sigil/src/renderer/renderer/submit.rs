use crate::hal::Color;
use crate::profile::count;

use super::WgpuRenderer;
#[cfg(not(target_arch = "wasm32"))]
use crate::profile::gpu::snapshot;
use crate::renderer::device_state::{DeviceState, ReadyState};
use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::layer_pool::LayerHandle;
use crate::renderer::scene_command::CommandScene;
use crate::renderer::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::renderer::split_encoder::SplitEncoder;
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

#[derive(Debug)]
pub struct WorldSubmission<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub depth: &'a wgpu::TextureView,
    pub view_proj: [[f32; 4]; 4],
    pub viewport: (f32, f32),
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
    count("gpu.render_passes");
    let _p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

pub fn submit_clear(
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
            stats,
        },
        scene,
    )
}

pub(super) fn render_frame_surface<F>(
    ready: &ReadyState,
    surface: &WgpuSurface,
    clear_color: crate::hal::Color,
    draw: F,
) -> Result<(), RendererError>
where
    F: FnOnce(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView),
{
    let Some((frame, view)) = acquire_frame(ready, surface)? else {
        return Ok(());
    };
    submit_clear(&ready.device, &ready.queue, &view, clear_color);
    draw(&ready.device, &ready.queue, &view);
    ready.queue.present(frame);
    Ok(())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn surface_text_scale(surface: &WgpuSurface, viewport: (f32, f32)) -> f32 {
    surface.config.width as f32 / viewport.0.max(1.0)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn render_command_scene_surface(
    ready: &mut ReadyState,
    surface: &WgpuSurface,
    scene: &CommandScene,
    clear_color: crate::hal::Color,
    viewport: (f32, f32),
    stats: &mut FrameStats,
) -> Result<(), RendererError> {
    let Some((frame, view)) = acquire_frame(ready, surface)? else {
        return Ok(());
    };
    let device = ready.device.clone();
    let queue = ready.queue.clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sigil-renderer surface frame encoder"),
    });
    record_clear(&mut encoder, &view, clear_color);
    let mut retained = Vec::new();
    let scale = surface_text_scale(surface, viewport);
    let format = surface.config.format;
    let mut encoder = SplitEncoder::new(&device, &queue, encoder);
    encoder.reuse_uploads(&mut ready.uploads);
    walk_screen(
        ready,
        &device,
        &queue,
        &mut encoder,
        &mut retained,
        &view,
        viewport,
        scale,
        format,
        scene,
        stats,
    )?;
    ready.uploads = Some(encoder.submit());
    queue.present(frame);
    if let Some(pool) = ready.layer_pool.as_mut() {
        for handle in retained {
            pool.release(handle);
        }
    }
    Ok(())
}

impl WgpuRenderer {
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
        if let Err(error) = self.prepare_screen(target, &mut encoder) {
            target.queue.submit(core::iter::once(encoder.finish()));
            return Err(error);
        }
        let mut split = SplitEncoder::new(target.device, target.queue, encoder);
        if let DeviceState::Ready(ready) = &mut self.state
            && ready.device == *target.device
        {
            split.reuse_uploads(&mut ready.uploads);
        }
        let result = self.walk_screen_commands(target, scene, &mut split);
        let uploads = split.submit();
        if let DeviceState::Ready(ready) = &mut self.state
            && ready.device == *target.device
        {
            ready.uploads = Some(uploads);
            #[cfg(not(target_arch = "wasm32"))]
            snapshot(&ready.instance);
        }
        result
    }

    pub fn record_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
        scene: &CommandScene,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RendererError> {
        self.prepare_screen(target, encoder)?;
        let real = std::mem::replace(
            encoder,
            target
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer record_screen placeholder"),
                }),
        );
        let mut split = SplitEncoder::new(target.device, target.queue, real);
        let result = self.walk_screen_commands(target, scene, &mut split);
        *encoder = split.into_inner();
        #[cfg(not(target_arch = "wasm32"))]
        if self.device().is_some_and(|device| device == target.device) {
            snapshot(self.instance());
        }
        result
    }

    fn prepare_screen(
        &mut self,
        target: &ScreenSubmission<'_>,
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
