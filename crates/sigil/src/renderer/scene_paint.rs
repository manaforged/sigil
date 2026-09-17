#![allow(clippy::redundant_pub_crate)]

use wgpu::util::DeviceExt as _;

#[path = "rect_shader.rs"]
mod rect_shader;

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
    pub clip: Option<crate::hal::Rect>,
    pub scale: f32,
}

pub(crate) struct ScenePipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    unit_quad: wgpu::Buffer,
}

impl ScenePipeline {
    #[allow(
        clippy::too_many_lines,
        reason = "one sequential pass; splitting would scatter the ordering it documents"
    )]
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sigil-renderer scene paint shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sigil-renderer scene uniforms layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sigil-renderer scene pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sigil-renderer scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(UNIT_QUAD_LAYOUT), Some(INSTANCE_LAYOUT)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let unit_quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sigil-renderer scene unit quad"),
            contents: bytemuck::cast_slice(&UNIT_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            format,
            pipeline,
            bind_group_layout,
            unit_quad,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn uniforms_bind_group(
        &self,
        device: &wgpu::Device,
        uniform_buffer: wgpu::BufferBinding<'_>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sigil-renderer scene uniforms bind group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer),
            }],
        })
    }

    pub(crate) fn paint_uploaded(
        &self,
        ctx: PaintCtx<'_>,
        instances: UploadedRange,
        uniform: wgpu::BufferBinding<'_>,
    ) {
        let PaintCtx {
            device,
            encoder,
            view,
            viewport,
            clip,
            scale,
            ..
        } = ctx;
        let bind_group = self.uniforms_bind_group(device, uniform);

        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer scene paint pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        if !crate::renderer::scissor::apply(pass, clip, scale, viewport) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instances.count);
    }
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
    array_stride: core::mem::size_of::<RectInstance>() as u64,
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

#[cfg(test)]
#[path = "scene_paint_tests.rs"]
mod tests;
