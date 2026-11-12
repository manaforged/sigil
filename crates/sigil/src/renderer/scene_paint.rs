#![allow(clippy::redundant_pub_crate)]

#[path = "rect_shader.rs"]
mod rect_shader;

use crate::renderer::fill_paint::setup::{QuadDesc, UniformLabels, UniformQuad};
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

pub(crate) type ScenePipeline = UniformQuad;

pub(crate) fn scene_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> ScenePipeline {
    UniformQuad::new(
        device,
        format,
        UniformLabels {
            layout: "sigil-renderer scene uniforms layout",
            group: "sigil-renderer scene uniforms bind group",
            pass: "sigil-renderer scene paint pass",
        },
        QuadDesc {
            label: "sigil-renderer scene",
            wgsl: RECT_SHADER,
            groups: &[],
            instance: INSTANCE_LAYOUT,
            fs_entry: "fs_main",
            blend: wgpu::BlendState::ALPHA_BLENDING,
        },
    )
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
