#![allow(clippy::redundant_pub_crate)]
//! Drop-shadow paint pipeline using Evan Wallace's rounded-rect shadow form.

use wgpu::util::DeviceExt as _;

#[path = "shadow_shader.rs"]
mod shadow_shader;

use crate::frame_upload::UploadedRange;
use crate::scene_instance::{ShadowInstance, srgb_channel_to_linear};
use crate::scene_paint::PaintCtx;
use shadow_shader::SHADOW_SHADER;

/// Selects whether a shadow is painted outside or inside its casting surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShadowKind {
    /// Paint a regular drop shadow outside the surface.
    #[default]
    Outer,
    /// Paint an inset shadow clipped to the surface.
    Inner,
    /// Paint a radial outer glow using a bright caller-supplied color.
    ///
    /// Structurally identical to [`ShadowKind::Outer`] but the caller supplies
    /// a bright tint (e.g. an accent color). The same Evan Wallace erf
    /// integration is used; the "halo" effect comes from the bright RGB values
    /// rather than any special blend math. Offset is always `[0, 0]` for a
    /// pure radial halo — use [`Shadow::glow`] to construct.
    Glow,
}

impl ShadowKind {
    pub(crate) const fn as_f32(self) -> f32 {
        match self {
            Self::Outer => 0.0,
            Self::Inner => 1.0,
            Self::Glow => 2.0,
        }
    }
}

/// Shadow styling carried by `SceneCommand::DrawShadow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Straight RGBA channels in `[0, 1]`.
    pub color: [f32; 4],
    /// Shadow offset in logical pixels.
    pub offset: [f32; 2],
    /// Gaussian sigma in logical pixels.
    pub blur: f32,
    /// Signed spread in logical pixels.
    pub spread: f32,
    /// Whether this shadow is outside or inside the casting surface.
    pub kind: ShadowKind,
}

impl Shadow {
    /// Create a regular outer drop shadow.
    #[must_use]
    pub const fn new(offset: [f32; 2], blur: f32, spread: f32, color: [f32; 4]) -> Self {
        Self {
            color,
            offset,
            blur,
            spread,
            kind: ShadowKind::Outer,
        }
    }

    /// Create a radial glow halo from an sRGB color.
    ///
    /// The offset is fixed at `[0, 0]` so the glow radiates equally in all
    /// directions. Supply a bright `color` (e.g. an accent tint) — the shader
    /// applies the same Evan Wallace coverage math as [`ShadowKind::Outer`] and
    /// multiplies by the caller-supplied color to produce the halo.
    #[must_use]
    pub fn glow(color: sigil_style::Rgba8, spread: f32, blur: f32) -> Self {
        let c = [
            srgb_channel_to_linear(color.r),
            srgb_channel_to_linear(color.g),
            srgb_channel_to_linear(color.b),
            f32::from(color.a) / 255.0,
        ];
        Self::new([0.0, 0.0], blur, spread, c).with_kind(ShadowKind::Glow)
    }

    /// Return this shadow with the given paint kind.
    #[must_use]
    pub const fn with_kind(mut self, kind: ShadowKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Wgpu resources for painting rounded-rectangle drop shadows.
///
/// Instance + uniform buffers are allocated fresh per call (not persistent
/// `queue.write_buffer` targets) so draws batched into one submission stay
/// independent — see [`crate::scene_paint::ScenePipeline`].
pub(crate) struct ShadowPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    unit_quad: wgpu::Buffer,
}

impl ShadowPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = shader(device);
        let uniform_layout = uniform_layout(device);
        let pipeline = pipeline(device, format, &shader, &uniform_layout);
        let unit_quad = unit_quad(device);
        Self {
            format,
            pipeline,
            uniform_layout,
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
        let bind_group = bind_group(device, &self.uniform_layout, uniform);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer shadow paint pass"),
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
        if !crate::scissor::apply(&mut pass, clip, scale, viewport) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instances.count);
    }
}

fn shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sigil-renderer shadow paint shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
    })
}

fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
    uniform_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sigil-renderer shadow pipeline layout"),
        bind_group_layouts: &[Some(uniform_layout)],
        immediate_size: 0,
    });
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sigil-renderer shadow pipeline"),
        layout: Some(&layout),
        vertex: vertex_state(shader),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn vertex_state(shader: &wgpu::ShaderModule) -> wgpu::VertexState<'_> {
    wgpu::VertexState {
        module: shader,
        entry_point: Some("vs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        buffers: &[UNIT_QUAD_LAYOUT, INSTANCE_LAYOUT],
    }
}

fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sigil-renderer shadow uniforms layout"),
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

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sigil-renderer shadow uniforms bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

fn unit_quad(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sigil-renderer shadow unit quad"),
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
    array_stride: core::mem::size_of::<ShadowInstance>() as u64,
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
#[path = "shadow_paint_glow_tests.rs"]
mod tests;
