use wgpu::util::DeviceExt as _;

use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::split_encoder::SplitEncoder;

const VERTEX_COUNT: u32 = 6;

const VERTICES: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

const UNIT_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: (core::mem::size_of::<f32>() as u64) * 2,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

pub(crate) struct QuadDesc<'a> {
    pub(crate) label: &'a str,
    pub(crate) wgsl: &'a str,
    pub(crate) groups: &'a [Option<&'a wgpu::BindGroupLayout>],
    pub(crate) instance: wgpu::VertexBufferLayout<'a>,
    pub(crate) fs_entry: &'a str,
    pub(crate) blend: wgpu::BlendState,
}

pub(crate) struct QuadPipeline {
    pub(crate) format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    unit_quad: wgpu::Buffer,
}

impl QuadPipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        desc: QuadDesc<'_>,
    ) -> Self {
        let label = desc.label;
        let (shader, layout) = shader_and_layout(device, &desc);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label} pipeline")),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(UNIT_LAYOUT), Some(desc.instance)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(desc.fs_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(desc.blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            format,
            pipeline,
            unit_quad: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{label} unit quad")),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a draw call names its pass, groups, offsets and instances"
    )]
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        groups: &[&wgpu::BindGroup],
        first_offsets: &[u32],
        instances: wgpu::BufferSlice<'_>,
        count: u32,
    ) {
        pass.set_pipeline(&self.pipeline);
        for (index, group) in (0_u32..).zip(groups) {
            let offsets: &[u32] = if index == 0 { first_offsets } else { &[] };
            pass.set_bind_group(index, *group, offsets);
        }
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instances);
        pass.draw(0..VERTEX_COUNT, 0..count);
    }
}

fn shader_and_layout(
    device: &wgpu::Device,
    desc: &QuadDesc<'_>,
) -> (wgpu::ShaderModule, wgpu::PipelineLayout) {
    let label = desc.label;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{label} shader")),
        source: wgpu::ShaderSource::Wgsl(desc.wgsl.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} pipeline layout")),
        bind_group_layouts: desc.groups,
        immediate_size: 0,
    });
    (shader, layout)
}

pub(crate) fn begin_pass<'e>(
    encoder: &'e mut SplitEncoder,
    label: &'static str,
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> &'e mut wgpu::RenderPass<'static> {
    encoder.begin_render_pass(
        label,
        &wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        },
    )
}

pub(crate) fn clipped_pass<'e>(
    ctx: &'e mut PaintCtx<'_>,
    label: &'static str,
) -> Option<&'e mut wgpu::RenderPass<'static>> {
    let pass = begin_pass(ctx.encoder, label, ctx.view, wgpu::LoadOp::Load);
    crate::renderer::scissor::apply(pass, ctx.clip, ctx.scale, ctx.viewport).then_some(pass)
}

pub(crate) fn buffer_layout(
    device: &wgpu::Device,
    label: &str,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub(crate) fn buffer_group(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    buffer: wgpu::BufferBinding<'_>,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(buffer),
        }],
    })
}

pub(crate) fn texture_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
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

pub(crate) fn texture_group(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
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

pub(crate) fn linear_sampler(device: &wgpu::Device, label: &str) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}
