#![allow(clippy::cast_precision_loss, clippy::redundant_pub_crate)]

use bytemuck::{Pod, Zeroable};
use std::slice::from_ref;
use wgpu::util::DeviceExt as _;

#[path = "layer_composite/shader.rs"]
mod shader;

use crate::hal::Rect;
use crate::profile::{count, span};
use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::layer_pool::LayerPool;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::scissor::apply;
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
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    sampler: wgpu::Sampler,
    unit_quad: wgpu::Buffer,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sigil-renderer layer composite shader"),
            source: wgpu::ShaderSource::Wgsl(LAYER_SHADER.into()),
        });
        let uniform_layout = uniform_layout(device);
        let texture_layout = texture_layout(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sigil-renderer layer composite sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            format,
            pipeline: pipeline(device, format, &shader, &uniform_layout, &texture_layout),
            uniform_layout,
            uniforms: None,
            texture_layout,
            textures: Vec::new(),
            sampler,
            unit_quad: unit_quad(device),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
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
        draw(self, &mut ctx, &instances, offset, texture);
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
                let group = texture_bind_group(device, &self.texture_layout, view, &self.sampler);
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

fn draw(
    pipeline: &LayerCompositePipeline,
    ctx: &mut PaintCtx<'_>,
    instances: &UploadedRange,
    uniform_offset: u32,
    texture: usize,
) {
    let (_, uniform_bg) = pipeline
        .uniforms
        .as_ref()
        .expect("layer uniforms initialized");
    let (_, texture_bg) = pipeline
        .textures
        .get(texture)
        .expect("layer texture initialized");
    let pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("sigil-renderer layer composite pass"),
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
    if !apply(pass, ctx.clip, ctx.scale, ctx.viewport) {
        return;
    }
    pass.set_pipeline(&pipeline.pipeline);
    pass.set_bind_group(0, uniform_bg, &[uniform_offset]);
    pass.set_bind_group(1, texture_bg, &[]);
    pass.set_vertex_buffer(0, pipeline.unit_quad.slice(..));
    pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes.clone()));
    pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..1);
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
    uniform_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sigil-renderer layer composite pipeline layout"),
        bind_group_layouts: &[Some(uniform_layout), Some(texture_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sigil-renderer layer composite pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(UNIT_QUAD_LAYOUT), Some(INSTANCE_LAYOUT)],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
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

fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer layer texture layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
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

fn texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    count(crate::profile::Counter::AllocCompositeTextureBindGroups);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer layer texture bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn unit_quad(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer layer unit quad"),
        contents: bytemuck::cast_slice(&UNIT_QUAD_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

const UNIT_QUAD_VERTEX_COUNT: u32 = 6;
const UNIT_QUAD_VERTICES: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

const UNIT_QUAD_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: (core::mem::size_of::<f32>() as u64) * 2,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

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
