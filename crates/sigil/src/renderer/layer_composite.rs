#![allow(clippy::cast_precision_loss)]

use bytemuck::{Pod, Zeroable};
use std::slice::from_ref;

#[path = "layer_composite/shader.rs"]
mod shader;

use crate::geometry::Rect;
use crate::profile::{count, span};
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, clipped_pass, linear_sampler, texture_group, texture_layout,
};
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::{CornerRadii, Transform};
use shader::LAYER_SHADER;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayerUniforms {
    viewport: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct LayerInstance {
    dest: [f32; 4],
    clip_radii: [f32; 4],
    uv_max: [f32; 2],
    opacity: f32,
    transform_x: [f32; 3],
    transform_y: [f32; 3],
    round: [f32; 4],
}

pub(crate) struct LayerCompositePipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    sampler: wgpu::Sampler,
}

impl LayerCompositePipeline {
    pub(crate) fn for_format<'a>(
        pipelines: &'a mut Vec<Self>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> &'a mut Self {
        let idx = pipelines
            .iter()
            .position(|p| p.format() == format)
            .unwrap_or_else(|| {
                let _span = span(crate::profile::Phase::RenderCompositePipeline);
                count(crate::profile::Counter::GpuCompositePipelines);
                pipelines.push(Self::new(device, format));
                pipelines.len() - 1
            });
        pipelines
            .get_mut(idx)
            .expect("layer pipeline index was just constructed from the vec")
    }

    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = uniform_layout(device);
        let texture_layout = texture_layout(device, "sigil-renderer layer texture layout");
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer layer composite",
                wgsl: LAYER_SHADER,
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
            textures: Vec::new(),
            sampler: linear_sampler(device, "sigil-renderer layer composite sampler"),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn retain(&mut self, pool: &LayerPool) {
        self.textures.retain(|(view, _)| pool.contains(view));
    }

    pub(crate) fn composite(&mut self, mut ctx: PaintCtx<'_>, request: CompositeRequest<'_>) {
        let instance = LayerInstance {
            dest: [
                request.dest.origin.x,
                request.dest.origin.y,
                request.dest.width,
                request.dest.height,
            ],
            clip_radii: request
                .clip_radii
                .normalized_for(request.round.width, request.round.height),
            uv_max: [
                request.source_extent.0 as f32 / request.texture_size.0.max(1) as f32,
                request.source_extent.1 as f32 / request.texture_size.1.max(1) as f32,
            ],
            opacity: request.opacity.clamp(0.0, 1.0),
            transform_x: request.transform[0],
            transform_y: request.transform[1],
            round: [
                request.round.width.mul_add(0.5, request.round.origin.x),
                request.round.height.mul_add(0.5, request.round.origin.y),
                request.round.width * 0.5,
                request.round.height * 0.5,
            ],
        };
        let offset = self.uniform(&mut ctx);
        let instances = ctx.encoder.upload(from_ref(&instance));
        let texture = self.texture(ctx.device, request.layer_view);
        let (_, uniform_bg) = self.uniforms.as_ref().expect("layer uniforms initialized");
        let (_, texture_bg) = self
            .textures
            .get(texture)
            .expect("layer texture initialized");
        if let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer layer composite pass") {
            self.quad.draw(
                pass,
                &[uniform_bg, texture_bg],
                &[offset],
                instances.buffer.slice(instances.bytes),
                1,
            );
        }
    }

    fn uniform(&mut self, ctx: &mut PaintCtx<'_>) -> u32 {
        let (buffer, offset, _) = ctx.encoder.uniform(&LayerUniforms {
            viewport: [ctx.viewport.0.max(1.0), ctx.viewport.1.max(1.0)],
            _pad: [0.0, 0.0],
        });
        if self
            .uniforms
            .as_ref()
            .is_none_or(|(cached, _)| *cached != buffer)
        {
            let group = uniform_bind_group(ctx.device, &self.uniform_layout, &buffer);
            self.uniforms = Some((buffer, group));
        }
        offset
    }

    fn texture(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) -> usize {
        self.textures
            .iter()
            .position(|(cached, _)| cached == view)
            .unwrap_or_else(|| {
                count(crate::profile::Counter::AllocCompositeTextureBindGroups);
                let group = texture_group(
                    device,
                    "sigil-renderer layer texture bind group",
                    &self.texture_layout,
                    view,
                    &self.sampler,
                );
                self.textures.push((view.clone(), group));
                self.textures.len() - 1
            })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CompositeRequest<'a> {
    pub(crate) layer_view: &'a wgpu::TextureView,
    pub(crate) dest: Rect,
    pub(crate) round: Rect,
    pub(crate) texture_size: (u32, u32),
    pub(crate) source_extent: (u32, u32),
    pub(crate) opacity: f32,
    pub(crate) clip_radii: CornerRadii,
    pub(crate) transform: Transform,
}

fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer layer uniforms layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size_of::<LayerUniforms>() as u64),
            },
            count: None,
        }],
    })
}

fn uniform_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    count(crate::profile::Counter::AllocCompositeUniformBindGroups);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer layer uniforms bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: wgpu::BufferSize::new(size_of::<LayerUniforms>() as u64),
            }),
        }],
    })
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<LayerInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x2,
        4 => Float32,
        5 => Float32x3,
        6 => Float32x3,
        7 => Float32x4,
    ],
};
