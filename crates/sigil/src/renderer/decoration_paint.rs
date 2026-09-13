#![allow(clippy::redundant_pub_crate)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_command::{DecorationKind, DrawDecoration};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;

const SHADER_SRC: &str = include_str!("decoration_paint.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct DecorationInstance {
    pub(crate) band: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) params: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<DecorationInstance>() == 48);

const UNIT_QUAD_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: (core::mem::size_of::<f32>() as u64) * 2,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<DecorationInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
    ],
};

const UNIT_QUAD_VERTICES: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

pub(crate) struct DecorationPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    unit_quad: wgpu::Buffer,
}

impl DecorationPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = make_shader(device);
        let bgl = make_bgl(device);
        let pipeline = make_pipeline(device, format, &shader, &bgl);
        let unit_quad = make_unit_quad(device);
        Self {
            format,
            pipeline,
            bgl,
            unit_quad,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn paint_uploaded(
        &self,
        ctx: PaintCtx<'_>,
        instances: UploadedRange,
        uniform: &wgpu::Buffer,
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
        let bind_group = make_bind_group(device, &self.bgl, uniform);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer decoration paint pass"),
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
        if !crate::renderer::scissor::apply(&mut pass, clip, scale, viewport) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes));
        pass.draw(0..6, 0..instances.count);
    }
}

fn make_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sigil-renderer decoration shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
    })
}

fn make_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer decoration uniforms layout"),
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
    })
}

fn make_bind_group(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer decoration bind group"),
        layout: bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    })
}

fn make_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
    bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sigil-renderer decoration pipeline layout"),
        bind_group_layouts: &[Some(bgl)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sigil-renderer decoration pipeline"),
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

fn make_unit_quad(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer decoration unit quad"),
        contents: bytemuck::cast_slice(&UNIT_QUAD_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub(crate) fn decoration_instance(
    draw: &DrawDecoration,
    opacity_scale: f32,
    space: BlendSpace,
) -> DecorationInstance {
    let opacity = (f32::from(draw.opacity) / 10_000.0) * opacity_scale;
    let kind_f32 = match draw.kind {
        DecorationKind::Underline => 0.0_f32,
        DecorationKind::Strikethrough => 1.0_f32,
        DecorationKind::WavyUnderline => 2.0_f32,
    };
    DecorationInstance {
        band: [
            draw.band.origin.x,
            draw.band.origin.y,
            draw.band.width,
            draw.band.height,
        ],
        color: space.rgba(draw.color),
        params: [opacity, draw.period, draw.line_width, kind_f32],
    }
}

#[cfg(test)]
#[path = "decoration_paint_tests.rs"]
mod tests;
