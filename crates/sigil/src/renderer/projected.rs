#![allow(
    clippy::cast_precision_loss,
    reason = "GPU texture dimensions fit exactly in f32"
)]

use bytemuck::{Pod, Zeroable};
use std::slice::from_ref;

#[path = "projected/frame.rs"]
mod frame;
#[path = "projected/shader.rs"]
mod shader;

pub(crate) use frame::ProjectedFrame;

use crate::geometry::Rect;
use crate::profile::count;
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, clipped_pass, linear_sampler, texture_group, texture_layout,
};
use crate::renderer::layer_composite::{uniform_layout, uv_max, viewport_uniform};
use crate::renderer::scene_command::walk::pipeline_for;
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::command::Transform3D;
use shader::PROJECTED_SHADER;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ProjectedInstance {
    dest: [f32; 4],
    uv_max: [f32; 2],
    opacity: f32,
    row_x: [f32; 3],
    row_y: [f32; 3],
    row_w: [f32; 3],
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectedRequest<'a> {
    pub(crate) layer_view: &'a wgpu::TextureView,
    pub(crate) dest: Rect,
    pub(crate) texture_size: (u32, u32),
    pub(crate) source_extent: (u32, u32),
    pub(crate) opacity: f32,
    pub(crate) transform: Transform3D,
}

pub(crate) struct ProjectedPipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl ProjectedPipeline {
    pub(crate) fn for_format<'a>(
        pipelines: &'a mut Vec<Self>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> &'a mut Self {
        pipeline_for(pipelines, device, format, (Self::format, Self::new))
    }

    const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        count(crate::profile::Counter::GpuCompositePipelines);
        let uniform_layout = uniform_layout(device, "sigil-renderer projected uniforms layout");
        let texture_layout = texture_layout(device, "sigil-renderer projected texture layout");
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer projected composite",
                wgsl: PROJECTED_SHADER,
                groups: &[Some(&uniform_layout), Some(&texture_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
            uniforms: None,
            texture_layout,
            sampler: linear_sampler(device, "sigil-renderer projected sampler"),
        }
    }

    pub(crate) fn composite(&mut self, mut ctx: PaintCtx<'_>, request: ProjectedRequest<'_>) {
        let [row_x, row_y, row_w] = request.transform.homography();
        let instance = ProjectedInstance {
            dest: [
                request.dest.origin.x,
                request.dest.origin.y,
                request.dest.width,
                request.dest.height,
            ],
            uv_max: uv_max(request.source_extent, request.texture_size),
            opacity: request.opacity.clamp(0.0, 1.0),
            row_x,
            row_y,
            row_w,
        };
        let offset = self.uniform(&mut ctx);
        let instances = ctx.encoder.upload(from_ref(&instance));
        count(crate::profile::Counter::AllocCompositeTextureBindGroups);
        let texture = texture_group(
            ctx.device,
            "sigil-renderer projected texture bind group",
            &self.texture_layout,
            request.layer_view,
            &self.sampler,
        );
        let Some((_, uniform_group)) = self.uniforms.as_ref() else {
            return;
        };
        if let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer projected composite pass") {
            self.quad.draw(
                pass,
                &[uniform_group, &texture],
                &[offset],
                instances.buffer.slice(instances.bytes),
                1,
            );
        }
    }

    fn uniform(&mut self, ctx: &mut PaintCtx<'_>) -> u32 {
        viewport_uniform(
            ctx,
            &self.uniform_layout,
            "sigil-renderer projected uniforms bind group",
            &mut self.uniforms,
        )
    }
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<ProjectedInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x2,
        3 => Float32,
        4 => Float32x3,
        5 => Float32x3,
        6 => Float32x3,
    ],
};
