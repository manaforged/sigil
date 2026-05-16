#![allow(clippy::redundant_pub_crate)]
//! Pattern + gradient fill rounded-rectangle paint pipeline.
//!
//! This module owns the `FillPipeline` which handles all three fill kinds:
//! `FILL_KIND_GRADIENT` (0), `FILL_KIND_SLASH` (1), and `FILL_KIND_CHECKER`
//! (2). The WGSL fragment shader branches on the `fill_kind` discriminator
//! packed into `stop_range.z`.

use crate::scene_instance::{
    FillInstance, GpuStop, SceneUniforms, fill_instance_stride, gpu_stop_stride,
};
use crate::scene_paint::PaintCtx;

mod setup;

use setup::{
    UNIT_QUAD_VERTEX_COUNT, doubled_capacity, instance_buffer, instance_slice, pipeline,
    queue_viewport, shader, stops_bind_group, stops_buffer, stops_layout, uniform_bind_group,
    uniform_buffer, uniform_layout, unit_quad,
};

/// Wgpu resources for painting fill rounded rectangles (gradient + patterns).
pub(crate) struct FillPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    stops_layout: wgpu::BindGroupLayout,
    stops_bind_group: wgpu::BindGroup,
    stops_buffer: wgpu::Buffer,
    unit_quad: wgpu::Buffer,
    instance_cap: u64,
    instance_buffer: wgpu::Buffer,
    stops_cap: u64,
}

impl FillPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = shader(device);
        let uniform_layout = uniform_layout(device);
        let stops_layout = stops_layout(device);
        let uniform_buffer = uniform_buffer(device);
        let uniform_bind_group = uniform_bind_group(device, &uniform_layout, &uniform_buffer);
        let initial_cap = 64_u64;
        let stops_buffer = stops_buffer(device, initial_cap);
        let stops_bind_group = stops_bind_group(device, &stops_layout, &stops_buffer);
        let pipeline = pipeline(device, format, &shader, &uniform_layout, &stops_layout);
        let unit_quad = unit_quad(device);
        let instance_buffer = instance_buffer(device, initial_cap);
        Self {
            format,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            stops_layout,
            stops_bind_group,
            stops_buffer,
            unit_quad,
            instance_cap: initial_cap,
            instance_buffer,
            stops_cap: initial_cap,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn paint_instances(
        &mut self,
        ctx: PaintCtx<'_>,
        instances: &[FillInstance],
        stops: &[GpuStop],
    ) {
        if instances.is_empty() {
            return;
        }
        // Patterns don't use stops but the buffer must be non-empty for the
        // storage binding. Provide a one-element sentinel when stops is empty.
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

        let mut ctx = ctx;
        self.upload(ctx.device, ctx.queue, instances, stops_slice);
        self.draw(&mut ctx, u32::try_from(instances.len()).unwrap_or(u32::MAX));
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[FillInstance],
        stops: &[GpuStop],
    ) {
        self.ensure_instance_capacity(device, instances.len() as u64);
        self.ensure_stops_capacity(device, stops.len() as u64);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&SceneUniforms {
                viewport: [1.0, 1.0],
                _pad: [0.0, 0.0],
            }),
        );
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        queue.write_buffer(&self.stops_buffer, 0, bytemuck::cast_slice(stops));
    }

    fn draw(&self, ctx: &mut PaintCtx<'_>, instance_count: u32) {
        queue_viewport(&self.uniform_buffer, ctx.queue, ctx.viewport);
        let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer fill paint pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ctx.view,
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
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_bind_group(1, &self.stops_bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instance_slice(&self.instance_buffer, instance_count));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instance_count);
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: u64) {
        if required <= self.instance_cap {
            return;
        }
        let new_cap = doubled_capacity(self.instance_cap, required);
        self.instance_buffer = instance_buffer(device, new_cap);
        self.instance_cap = new_cap;
    }

    fn ensure_stops_capacity(&mut self, device: &wgpu::Device, required: u64) {
        if required <= self.stops_cap {
            return;
        }
        let new_cap = doubled_capacity(self.stops_cap, required);
        self.stops_buffer = stops_buffer(device, new_cap);
        self.stops_bind_group = stops_bind_group(device, &self.stops_layout, &self.stops_buffer);
        self.stops_cap = new_cap;
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
