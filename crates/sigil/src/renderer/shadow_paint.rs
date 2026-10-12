#![allow(clippy::redundant_pub_crate)]

#[path = "shadow_shader.rs"]
mod shadow_shader;

use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, buffer_group, buffer_layout, clipped_pass,
};
use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_instance::ShadowInstance;
use crate::renderer::scene_paint::PaintCtx;
use shadow_shader::SHADOW_SHADER;

pub(crate) use crate::scene::shadow::{Shadow, ShadowKind};

impl ShadowKind {
    pub(crate) const fn as_f32(self) -> f32 {
        match self {
            Self::Outer => 0.0,
            Self::Inner => 1.0,
            Self::Glow => 2.0,
        }
    }
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<ShadowInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
    ],
};

pub(crate) struct ShadowPipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
}

impl ShadowPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = buffer_layout(
            device,
            "sigil-renderer shadow uniforms layout",
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
        );
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer shadow",
                wgsl: SHADOW_SHADER,
                groups: &[Some(&uniform_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
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
            "sigil-renderer shadow uniforms bind group",
            &self.uniform_layout,
            uniform,
        );
        let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer shadow paint pass") else {
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

#[cfg(test)]
#[path = "shadow_paint_glow_tests.rs"]
mod tests;
