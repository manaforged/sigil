#![allow(clippy::redundant_pub_crate)]

#[path = "rect_shader.rs"]
mod rect_shader;

use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, buffer_group, buffer_layout, clipped_pass,
};
use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_instance::RectInstance;
use crate::renderer::split_encoder::SplitEncoder;
use rect_shader::RECT_SHADER;

pub(crate) struct PaintCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut SplitEncoder,
    pub view: &'a wgpu::TextureView,
    pub viewport: (f32, f32),
    pub clip: Option<crate::geometry::Rect>,
    pub scale: f32,
}

pub(crate) struct ScenePipeline {
    quad: QuadPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ScenePipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bind_group_layout = buffer_layout(
            device,
            "sigil-renderer scene uniforms layout",
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
        );
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer scene",
                wgsl: RECT_SHADER,
                groups: &[Some(&bind_group_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            bind_group_layout,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn paint_uploaded(
        &self,
        mut ctx: PaintCtx<'_>,
        instances: UploadedRange,
        uniform: wgpu::BufferBinding<'_>,
    ) {
        let bind_group = buffer_group(
            ctx.device,
            "sigil-renderer scene uniforms bind group",
            &self.bind_group_layout,
            uniform,
        );
        let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer scene paint pass") else {
            return;
        };
        self.quad.draw(
            pass,
            &[&bind_group],
            &[],
            instances.buffer.slice(instances.bytes),
            instances.count,
        );
    }
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<RectInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
    ],
};

#[cfg(test)]
#[path = "scene_paint_tests.rs"]
mod tests;
