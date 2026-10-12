#![allow(clippy::redundant_pub_crate)]

use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_paint::PaintCtx;

pub(crate) mod setup;

use setup::{QuadDesc, QuadPipeline, buffer_group, buffer_layout, clipped_pass};

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<crate::renderer::scene_instance::FillInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Uint32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4,
        11 => Float32x4,
    ],
};

pub(crate) struct FillPipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    stops_layout: wgpu::BindGroupLayout,
}

impl FillPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = buffer_layout(
            device,
            "sigil-renderer fill paint uniform layout",
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
        );
        let stops_layout = buffer_layout(
            device,
            "sigil-renderer fill paint stops layout",
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BufferBindingType::Storage { read_only: true },
        );
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer fill paint",
                wgsl: include_str!("fill_paint.wgsl"),
                groups: &[Some(&uniform_layout), Some(&stops_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
            stops_layout,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn paint_uploaded(
        &self,
        mut ctx: PaintCtx<'_>,
        instances: UploadedRange,
        stops: &UploadedRange,
        uniform: wgpu::BufferBinding<'_>,
    ) {
        let uniform_bg = buffer_group(
            ctx.device,
            "sigil-renderer fill paint uniform bind group",
            &self.uniform_layout,
            uniform,
        );
        let stops_bg = buffer_group(
            ctx.device,
            "sigil-renderer fill paint stops bind group",
            &self.stops_layout,
            wgpu::BufferBinding {
                buffer: &stops.buffer,
                offset: stops.bytes.start,
                size: wgpu::BufferSize::new(stops.bytes.end - stops.bytes.start),
            },
        );
        let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer fill paint pass") else {
            return;
        };
        self.quad.draw(
            pass,
            &[&uniform_bg, &stops_bg],
            &[],
            instances.buffer.slice(instances.bytes),
            instances.count,
        );
    }
}

#[cfg(test)]
#[path = "fill_paint_tests.rs"]
mod tests;
