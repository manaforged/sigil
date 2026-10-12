use crate::profile::count;

use super::{SpriteInstance, SpriteUniforms};

pub(super) const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<SpriteInstance>() as u64,
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

pub(super) fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer sprite uniforms layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size_of::<SpriteUniforms>() as u64),
            },
            count: None,
        }],
    })
}

pub(super) fn uniform_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    count(crate::profile::Counter::AllocSpriteBindGroups);
    crate::renderer::fill_paint::setup::buffer_group(
        device,
        "sigil-renderer sprite uniforms bind group",
        layout,
        wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: wgpu::BufferSize::new(size_of::<SpriteUniforms>() as u64),
        },
    )
}
