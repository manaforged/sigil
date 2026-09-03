use sigil_hal::Color;

use crate::device_state::ReadyState;
use crate::error::RendererError;
use crate::frame_stats::FrameStats;
use crate::layer_pool::LayerHandle;
use crate::scene_command::CommandScene;
use crate::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::split_encoder::SplitEncoder;
use crate::surface::WgpuSurface;

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
    clear_color: sigil_hal::Color,
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
    clear_color: sigil_hal::Color,
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
    queue.submit(core::iter::once(encoder.into_inner().finish()));
    queue.present(frame);
    if let Some(pool) = ready.layer_pool.as_mut() {
        for handle in retained {
            pool.release(handle);
        }
    }
    Ok(())
}
