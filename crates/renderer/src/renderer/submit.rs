//! Screen/world submission target descriptors and shared submit helpers.

use sigil_hal::Color;

use crate::device_state::ReadyState;
use crate::error::RendererError;
use crate::frame_stats::FrameStats;
use crate::layer_pool::LayerHandle;
use crate::scene_command::CommandScene;
use crate::scene_command::walk::{WalkCtx, walk_command_scene};
use crate::surface::WgpuSurface;

/// Screen-space submission target. Orthographic, no depth, alpha-over.
///
/// Pass this to [`crate::WgpuRenderer::submit_screen`] to render UI geometry on top
/// of a frame. The renderer does not acquire or present a swapchain surface;
/// the caller owns the full frame graph and passes in an already-acquired view.
#[derive(Debug)]
pub struct ScreenSubmission<'a> {
    /// Device that owns `view`.
    pub device: &'a wgpu::Device,
    /// Queue paired with `device`.
    pub queue: &'a wgpu::Queue,
    /// Texture view to draw into.
    pub view: &'a wgpu::TextureView,
    /// Texture format used to build render pipelines.
    pub format: wgpu::TextureFormat,
    /// Logical viewport in pixels — matches scene-build coordinates.
    pub viewport: (f32, f32),
    /// Physical pixels per logical pixel of `view` — the target's DPI scale
    /// factor. `1.0` means logical == physical (`view` is exactly `viewport`
    /// pixels); `2.0` means `view` is a `2×` high-DPI target (`2 * viewport`
    /// pixels). Text draws multiply their glyph-coverage AA ramp by this so the
    /// ramp stays one *physical* pixel wide — sharp on high-DPI displays instead
    /// of blurry. Mirrors [`DrawParams::scale`][crate::DrawParams::scale].
    ///
    /// **Required, no default.** Embedders must state their target's DPI: a
    /// silent `1.0` renders text at logical scale into a physical texture, which
    /// is exactly the blur this field exists to prevent.
    pub scale: f32,
    /// Optional clear colour applied before drawing.
    /// `None` means the caller pre-clears or is compositing on top.
    pub clear: Option<Color>,
}

/// World-space submission target. View-projection transformed, depth-aware.
///
/// Pass this to [`crate::WgpuRenderer::submit_world`] to render depth-occluded UI
/// geometry (nameplates, damage numbers, 3D labels) into a world-space frame.
///
/// **R13 ships `submit_world` as a stub** — see its doc comment.
#[derive(Debug)]
pub struct WorldSubmission<'a> {
    /// Device that owns `view` and `depth`.
    pub device: &'a wgpu::Device,
    /// Queue paired with `device`.
    pub queue: &'a wgpu::Queue,
    /// Colour target to draw into.
    pub view: &'a wgpu::TextureView,
    /// Texture format for the colour target.
    pub format: wgpu::TextureFormat,
    /// Depth texture for occlusion testing. Read-only; the renderer samples
    /// it to discard fragments behind world geometry.
    pub depth: &'a wgpu::TextureView,
    /// 4×4 row-major view-projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Pixel-space viewport dimensions for the target.
    pub viewport: (f32, f32),
}

/// Acquire the next surface texture. Returns `Ok(None)` when the surface
/// is lost / outdated / timing-out — caller should return cleanly.
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

/// Record a clear-to-`color` pass into `encoder` without submitting.
/// The caller owns the submission — used by the encoder-recording paths so a
/// clear is sequenced with the scene's draws in one submission (DR-054).
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

/// Clear `view` to `color` by submitting a single render pass.
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

/// Record `scene` into `view` and the caller-owned `encoder` using the
/// format-keyed pipeline caches in `ready`. The caller submits `encoder`.
///
/// Shared by `submit_screen` / `record_screen` and the surface present path.
#[allow(clippy::too_many_arguments)]
pub(super) fn walk_screen(
    ready: &mut ReadyState,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
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
            // Caller-declared DPI: physical pixels per logical pixel of `view`.
            scale,
            format,
            scene_pipelines,
            fill_pipelines,
            shadow_pipelines,
            slug_pipelines,
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

/// Acquire surface frame, clear, run `draw`, and present.
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
    frame.present();
    Ok(())
}

/// Device pixel ratio: physical surface width divided by the logical viewport width — the
/// window scale factor (1.0 on standard displays, 2.0 on high-DPI). Drives the text
/// shader's coverage-ramp scaling so glyph edges stay one physical pixel wide.
#[allow(clippy::cast_precision_loss)]
fn surface_text_scale(surface: &WgpuSurface, viewport: (f32, f32)) -> f32 {
    surface.config.width as f32 / viewport.0.max(1.0)
}

/// Acquire surface frame, clear, walk `scene`, and present.
#[allow(clippy::too_many_arguments)]
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
    // Clone (Arc-backed, cheap) so `device`/`queue` can be passed alongside
    // `ready` without a double-mutable-borrow — mirrors `walk_screen`'s other
    // callers (`record_screen`).
    let device = ready.device.clone();
    let queue = ready.queue.clone();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sigil-renderer surface frame encoder"),
    });
    record_clear(&mut encoder, &view, clear_color);
    let mut retained = Vec::new();
    let scale = surface_text_scale(surface, viewport);
    let format = surface.config.format;
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
    queue.submit(core::iter::once(encoder.finish()));
    frame.present();
    if let Some(pool) = ready.layer_pool.as_mut() {
        for handle in retained {
            pool.release(handle);
        }
    }
    Ok(())
}
