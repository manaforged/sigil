#![allow(clippy::redundant_pub_crate)]

use bytemuck::{Pod, Zeroable};

use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, buffer_group, buffer_layout, clipped_pass,
};
use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_command::{DecorationKind, DrawDecoration, Transform};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;

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

pub(crate) struct DecorationPipeline {
    quad: QuadPipeline,
    bgl: wgpu::BindGroupLayout,
}

impl DecorationPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let bgl = buffer_layout(
            device,
            "sigil-renderer decoration uniforms layout",
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
        );
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer decoration",
                wgsl: include_str!("decoration_paint.wgsl"),
                groups: &[Some(&bgl)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self { quad, bgl }
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
            "sigil-renderer decoration bind group",
            &self.bgl,
            uniform,
        );
        let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer decoration paint pass") else {
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
