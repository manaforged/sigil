//! wgpu pipeline creation for the Slug text renderer — Phase 2.
//!
//! Bind group layout:
//!   binding 0 — `SurfaceParams` uniform (vertex + fragment)
//!   binding 1 — atlas storage buffer, all packed `BandData` (fragment)
//!
//! Vertex buffers:
//!   slot 0 — UV quad vertices (Float32x2, step = Vertex)
//!   slot 1 — per-glyph `GlyphInstance` (step = Instance):
//!     loc 1  origin       Float32x2   ( 0)
//!     loc 2  size         Float32x2   ( 8)
//!     loc 3  color        Float32x4   (16)
//!     loc 4  `atlas_offset` Uint32      (32)
//!     _pad                            (36) — stride = 40

/// Create the bind group layout shared across all frames.
pub(super) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("slug-bgl"),
        entries: &[
            // binding 0: SurfaceParams uniform (vertex + fragment)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // binding 1: atlas storage buffer (fragment only)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Per-vertex UV-quad attribute (slot 0).
const QUAD_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

/// Per-instance `GlyphInstance` attributes (slot 1, stride = 40 bytes).
///   1 origin (0)  2 size (8)  3 color (16)  4 `atlas_offset` (32)  5 polarity (36)
const INSTANCE_ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    1 => Float32x2,
    2 => Float32x2,
    3 => Float32x4,
    4 => Uint32,
    5 => Float32,
];

/// The Slug vertex-buffer layouts: UV quad (per-vertex) + `GlyphInstance` (per-instance).
const fn vertex_buffers<'a>() -> [wgpu::VertexBufferLayout<'a>; 2] {
    [
        wgpu::VertexBufferLayout {
            array_stride: 8, // 2 × f32
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &QUAD_ATTRS,
        },
        wgpu::VertexBufferLayout {
            array_stride: 40,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRS,
        },
    ]
}

/// Create the Slug render pipeline for `surface_format`.
pub(super) fn create_pipeline(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("slug-layout"),
        bind_group_layouts: &[Some(bgl)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("slug-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &vertex_buffers(),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
