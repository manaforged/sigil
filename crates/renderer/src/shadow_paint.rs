#![allow(clippy::redundant_pub_crate)]
//! Drop-shadow paint pipeline using Evan Wallace's rounded-rect shadow form.

use wgpu::util::DeviceExt as _;

#[path = "shadow_shader.rs"]
mod shadow_shader;

use crate::scene_instance::{
    SceneUniforms, ShadowInstance, shadow_instance_stride, srgb_channel_to_linear,
};
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
    pub fn glow(color: style::Rgba8, spread: f32, blur: f32) -> Self {
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
pub(crate) struct ShadowPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    unit_quad: wgpu::Buffer,
    instance_cap: u64,
    instance_buffer: wgpu::Buffer,
}

impl ShadowPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = shader(device);
        let uniform_layout = uniform_layout(device);
        let uniform_buffer = uniform_buffer(device);
        let bind_group = bind_group(device, &uniform_layout, &uniform_buffer);
        let pipeline = pipeline(device, format, &shader, &uniform_layout);
        let unit_quad = unit_quad(device);
        let instance_cap = 64_u64;
        let instance_buffer = instance_buffer(device, instance_cap);
        Self {
            format,
            pipeline,
            uniform_buffer,
            bind_group,
            unit_quad,
            instance_cap,
            instance_buffer,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn paint_instances(&mut self, ctx: PaintCtx<'_>, instances: &[ShadowInstance]) {
        if instances.is_empty() {
            return;
        }
        let mut ctx = ctx;
        self.upload(ctx.device, ctx.queue, instances);
        self.draw(&mut ctx, u32::try_from(instances.len()).unwrap_or(u32::MAX));
    }

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instances: &[ShadowInstance]) {
        self.ensure_instance_capacity(device, instances.len() as u64);
        queue_viewport(&self.uniform_buffer, queue, (1.0, 1.0));
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
    }

    fn draw(&self, ctx: &mut PaintCtx<'_>, instance_count: u32) {
        queue_viewport(&self.uniform_buffer, ctx.queue, ctx.viewport);
        let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer shadow paint pass"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instance_slice(&self.instance_buffer, instance_count));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instance_count);
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: u64) {
        if required <= self.instance_cap {
            return;
        }
        let new_cap = doubled_capacity(self.instance_cap, required);
        self.instance_buffer = instance_buffer(device, new_cap);
        self.instance_cap = new_cap;
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

fn uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil-renderer shadow uniforms"),
        size: u64::try_from(core::mem::size_of::<SceneUniforms>()).unwrap_or(0),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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

fn instance_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil-renderer shadow instance buffer"),
        size: capacity * shadow_instance_stride(),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn queue_viewport(buffer: &wgpu::Buffer, queue: &wgpu::Queue, viewport: (f32, f32)) {
    queue.write_buffer(
        buffer,
        0,
        bytemuck::bytes_of(&SceneUniforms {
            viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
            _pad: [0.0, 0.0],
        }),
    );
}

fn instance_slice(buffer: &wgpu::Buffer, instance_count: u32) -> wgpu::BufferSlice<'_> {
    buffer.slice(0..shadow_instance_stride() * u64::from(instance_count))
}

fn doubled_capacity(current: u64, required: u64) -> u64 {
    let mut new_cap = current.max(1);
    while new_cap < required {
        new_cap = new_cap.saturating_mul(2);
    }
    new_cap
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
mod tests {
    use pollster::FutureExt as _;

    use super::ShadowPipeline;
    use super::shadow_shader::SHADOW_SHADER;

    #[test]
    fn shadow_pipeline_constructs_against_real_device() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
        else {
            eprintln!("no adapter - skipping shadow-pipeline shader-validation test");
            return;
        };
        let Ok((device, _queue)) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("shadow-paint-test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .block_on()
        else {
            eprintln!("no device - skipping shadow-pipeline shader-validation test");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let pipeline = ShadowPipeline::new(&device, format);
        assert_eq!(pipeline.format(), format);
    }

    #[test]
    fn inner_shadow_shader_clips_and_inverts_coverage() {
        assert!(SHADOW_SHADER.contains("if shadow_kind >= 0.5"));
        assert!(SHADOW_SHADER.contains("if d_surface > 0.0"));
        assert!(SHADOW_SHADER.contains("let inner_spread = -spread"));
        assert!(SHADOW_SHADER.contains("coverage = 1.0 - shadow_coverage"));
    }

    include!("shadow_paint_glow_tests.rs");
}
