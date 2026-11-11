#![allow(clippy::redundant_pub_crate)]

use bytemuck::{Pod, Zeroable};

use crate::renderer::fill_paint::setup::{QuadDesc, UniformLabels, UniformQuad};
use crate::renderer::scene_command::{DecorationKind, DrawDecoration, Transform};
use crate::renderer::scene_instance::BlendSpace;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct DecorationInstance {
    pub(crate) band: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) axes: [f32; 4],
    pub(crate) offset: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<DecorationInstance>() == 80);

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<DecorationInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
    ],
};

pub(crate) type DecorationPipeline = UniformQuad;

pub(crate) fn decoration_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> DecorationPipeline {
    UniformQuad::new(
        device,
        format,
        UniformLabels {
            layout: "sigil-renderer decoration uniforms layout",
            group: "sigil-renderer decoration bind group",
            pass: "sigil-renderer decoration paint pass",
        },
        QuadDesc {
            label: "sigil-renderer decoration",
            wgsl: include_str!("decoration_paint.wgsl"),
            groups: &[],
            instance: INSTANCE_LAYOUT,
            fs_entry: "fs_main",
            blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        },
    )
}

pub(crate) fn decoration_instance(
    draw: &DrawDecoration,
    transform: Transform,
    opacity_scale: f32,
    space: BlendSpace,
) -> Option<DecorationInstance> {
    let geometry = draw.geometry()?;
    let opacity = (f32::from(draw.opacity) / 10_000.0) * opacity_scale;
    let kind_f32 = match draw.kind {
        DecorationKind::Underline => 0.0_f32,
        DecorationKind::Strikethrough => 1.0_f32,
        #[cfg(cpu_paint)]
        DecorationKind::WavyUnderline => 2.0_f32,
    };
    let band = geometry.band;
    let [[a, c, tx], [b, d, ty]] = transform;
    Some(DecorationInstance {
        band: [band.origin.x, band.origin.y, band.width, band.height],
        color: space.rgba(draw.color),
        params: [opacity, geometry.period, geometry.thickness, kind_f32],
        axes: [a, b, c, d],
        offset: [tx, ty, geometry.amplitude, 0.0],
    })
}

#[cfg(test)]
#[path = "decoration_paint_tests.rs"]
mod tests;
