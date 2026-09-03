use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct BlurUniforms {
    pub(super) viewport: [f32; 2],
    pub(super) _pad: [f32; 2],
}

const _: () = assert!(core::mem::size_of::<BlurUniforms>() == 16);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct BlurInstance {
    pub(super) dest: [f32; 4],
    pub(super) uv_max: [f32; 2],
    pub(super) opacity: f32,
    pub(super) _pad: f32,
}

const _: () = assert!(core::mem::size_of::<BlurInstance>() == 32);

pub(super) const UNIT_QUAD_VERTEX_COUNT: u32 = 6;

pub(super) const UNIT_QUAD_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: (core::mem::size_of::<f32>() as u64) * 2,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

pub(super) const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<BlurInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x2,
        3 => Float32,
    ],
};

pub(super) fn make_uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer blur uniforms layout"),
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

pub(super) fn make_texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer blur texture layout"),
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

pub(super) fn make_uniform_buffer(device: &wgpu::Device, uniforms: &BlurUniforms) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer blur uniforms"),
        contents: bytemuck::bytes_of(uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

pub(super) fn make_uniform_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer blur uniforms bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

pub(super) fn make_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer blur texture bind group"),
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

pub(super) fn make_unit_quad(device: &wgpu::Device) -> wgpu::Buffer {
    const VERTS: [[f32; 2]; 6] = [
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer blur unit quad"),
        contents: bytemuck::cast_slice(&VERTS),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub(super) fn make_instance_buffer(device: &wgpu::Device, instance: &BlurInstance) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer blur instance buffer"),
        contents: bytemuck::bytes_of(instance),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub(super) fn make_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fs_entry: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sigil-renderer blur pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(UNIT_QUAD_LAYOUT), Some(INSTANCE_LAYOUT)],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
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
