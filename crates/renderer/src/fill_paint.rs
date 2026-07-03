#![allow(clippy::redundant_pub_crate)]
//! Pattern + gradient fill rounded-rectangle paint pipeline.
//!
//! This module owns the `FillPipeline` which handles all three fill kinds:
//! `FILL_KIND_GRADIENT` (0), `FILL_KIND_SLASH` (1), and `FILL_KIND_CHECKER`
//! (2). The WGSL fragment shader branches on the `fill_kind` discriminator
//! packed into `stop_range.z`.

use crate::scene_instance::{FillInstance, GpuStop};
use crate::scene_paint::PaintCtx;

mod setup;

use setup::{
    UNIT_QUAD_VERTEX_COUNT, instance_buffer, pipeline, shader, stops_bind_group, stops_buffer,
    stops_layout, uniform_bind_group, uniform_buffer, uniform_layout, unit_quad,
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

    pub(crate) fn paint_instances(
        &self,
        ctx: PaintCtx<'_>,
        instances: &[FillInstance],
        stops: &[GpuStop],
    ) {
        if instances.is_empty() {
            return;
        }
        // Patterns don't use stops but the storage buffer must be non-empty.
        let sentinel = [GpuStop {
            offset_bits: 0,
            rgba01: 0,
            rgba23: 0,
            _pad: 0,
        }];
        let stops_slice = if stops.is_empty() {
            &sentinel[..]
        } else {
            stops
        };

        let PaintCtx {
            device,
            encoder,
            view,
            viewport,
            ..
        } = ctx;
        let instance_buf = instance_buffer(device, instances);
        let stops_buf = stops_buffer(device, stops_slice);
        let uniform_buf = uniform_buffer(device, viewport);
        let uniform_bg = uniform_bind_group(device, &self.uniform_layout, &uniform_buf);
        let stops_bg = stops_bind_group(device, &self.stops_layout, &stops_buf);
        let count = u32::try_from(instances.len()).unwrap_or(u32::MAX);

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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &uniform_bg, &[]);
        pass.set_bind_group(1, &stops_bg, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..count);
    }
}

#[cfg(test)]
mod tests {
    use pollster::FutureExt as _;

    use super::FillPipeline;

    #[test]
    fn fill_pipeline_constructs_against_real_device() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
        else {
            eprintln!("no adapter - skipping fill-pipeline shader-validation test");
            return;
        };
        let Ok((device, _queue)) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("fill-paint-test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .block_on()
        else {
            eprintln!("no device - skipping fill-pipeline shader-validation test");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let pipeline = FillPipeline::new(&device, format);
        assert_eq!(pipeline.format(), format);
    }
}
