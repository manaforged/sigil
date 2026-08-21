#![allow(clippy::redundant_pub_crate)]
//! Pattern + gradient fill rounded-rectangle paint pipeline.
//!
//! This module owns the `FillPipeline` which handles all three fill kinds:
//! `FILL_KIND_GRADIENT` (0), `FILL_KIND_SLASH` (1), and `FILL_KIND_CHECKER`
//! (2). The WGSL fragment shader branches on the `fill_kind` discriminator
//! packed into `stop_range.z`.

use crate::frame_upload::UploadedRange;
use crate::scene_paint::PaintCtx;

mod setup;

use setup::{
    UNIT_QUAD_VERTEX_COUNT, pipeline, shader, stops_bind_group, stops_layout, uniform_bind_group,
    uniform_layout, unit_quad,
};

/// Wgpu resources for painting fill rounded rectangles (gradient + patterns).
///
/// Instance / stops / uniform buffers are allocated fresh per call (not
/// persistent `queue.write_buffer` targets) so draws batched into one
/// submission stay independent — see [`crate::scene_paint::ScenePipeline`].
pub(crate) struct FillPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    stops_layout: wgpu::BindGroupLayout,
    unit_quad: wgpu::Buffer,
}

impl FillPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = shader(device);
        let uniform_layout = uniform_layout(device);
        let stops_layout = stops_layout(device);
        let pipeline = pipeline(device, format, &shader, &uniform_layout, &stops_layout);
        let unit_quad = unit_quad(device);
        Self {
            format,
            pipeline,
            uniform_layout,
            stops_layout,
            unit_quad,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn paint_uploaded(
        &self,
        ctx: PaintCtx<'_>,
        instances: UploadedRange,
        stops: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
    ) {
        let PaintCtx {
            device,
            encoder,
            view,
            viewport,
            clip,
            scale,
            ..
        } = ctx;
        let uniform_bg = uniform_bind_group(device, &self.uniform_layout, uniform);
        let stops_bg = stops_bind_group(device, &self.stops_layout, stops);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer fill paint pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if !crate::scissor::apply(&mut pass, clip, scale, viewport) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &uniform_bg, &[]);
        pass.set_bind_group(1, &stops_bg, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instances.count);
    }
}

#[cfg(test)]
#[path = "fill_paint_tests.rs"]
mod tests;
