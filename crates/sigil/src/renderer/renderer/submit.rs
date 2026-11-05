use crate::profile::count;
use crate::style::Color;

use crate::geometry::Rect;
use crate::renderer::device_state::ReadyState;
use crate::renderer::split_encoder::clear_value;
use crate::renderer::surface::WgpuSurface;
use crate::{Error, ErrorKind};

pub(super) fn acquire_frame(
    ready: &ReadyState,
    surface: &WgpuSurface,
) -> Result<Option<(wgpu::SurfaceTexture, wgpu::TextureView)>, Error> {
    let acquired = {
        let _wait = crate::profile::span(crate::profile::Phase::GpuWait);
        surface.surface.get_current_texture()
    };
    let frame = match acquired {
        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            surface.surface.configure(&ready.device, &surface.config);
            return Ok(None);
        }
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return Ok(None);
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            return Err(Error::fixed(
                ErrorKind::Gpu,
                "surface acquisition failed with a validation error",
            ));
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
