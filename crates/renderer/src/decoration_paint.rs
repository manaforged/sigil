#![allow(clippy::redundant_pub_crate)]
//! Text-decoration paint pipeline — underline / strikethrough / wavy underline.
//!
//! One pipeline per surface `TextureFormat`, cached in `ReadyState`.
//! The WGSL shader branches on a `kind` discriminant (0/1/2) packed into
//! the per-instance `params.w` slot.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use crate::scene_command::{DecorationKind, DrawDecoration};
use crate::scene_paint::PaintCtx;

// ── WGSL source ──────────────────────────────────────────────────────────────

const SHADER_SRC: &str = include_str!("decoration_paint.wgsl");

// ── GPU instance record ───────────────────────────────────────────────────────

/// Per-instance data uploaded for one decoration draw call.
///
/// Layout (largest → smallest, explicit pad to 16-byte boundary):
/// ```text
/// band:   [f32; 4]  — 16 bytes  (x, y, w, h)
/// color:  [f32; 4]  — 16 bytes  (linear RGBA)
/// params: [f32; 4]  — 16 bytes  (opacity, period, line_width, kind_f32)
/// ```
/// Total: 48 bytes.  All fields are vec4-aligned; no implicit padding needed.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct DecorationInstance {
    /// Band rect `[x, y, w, h]` in logical pixels.
    pub(crate) band: [f32; 4],
    /// Linear RGBA in `[0, 1]`.
    pub(crate) color: [f32; 4],
    /// `[opacity_01, period, line_width, kind_f32]`.
    pub(crate) params: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<DecorationInstance>() == 48);

const fn instance_stride() -> u64 {
    core::mem::size_of::<DecorationInstance>() as u64
}

// ── Uniform block ─────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DecUniforms {
    viewport: [f32; 2],
    _pad: [f32; 2],
}

// ── Vertex buffer layouts ─────────────────────────────────────────────────────

const UNIT_QUAD_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: (core::mem::size_of::<f32>() as u64) * 2,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
};

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<DecorationInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,  // band
        2 => Float32x4,  // color
        3 => Float32x4,  // params
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

// ── Pipeline ──────────────────────────────────────────────────────────────────

/// Wgpu resources for painting text decorations. One per surface format.
pub(crate) struct DecorationPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    unit_quad: wgpu::Buffer,
    instance_cap: u64,
    instance_buffer: wgpu::Buffer,
}

impl DecorationPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = make_shader(device);
        let bgl = make_bgl(device);
        let uniform_buffer = make_uniform_buffer(device);
        let bind_group = make_bind_group(device, &bgl, &uniform_buffer);
        let pipeline = make_pipeline(device, format, &shader, &bgl);
        let unit_quad = make_unit_quad(device);
        let initial_cap = 16_u64;
        let instance_buffer = make_instance_buffer(device, initial_cap);
        Self {
            format,
            pipeline,
            uniform_buffer,
            bind_group,
            unit_quad,
            instance_cap: initial_cap,
            instance_buffer,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: u64) {
        if required <= self.instance_cap {
            return;
        }
        let mut cap = self.instance_cap.max(1);
        while cap < required {
            cap = cap.saturating_mul(2);
        }
        self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sigil-renderer decoration instance buffer"),
            size: cap * instance_stride(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_cap = cap;
    }

    /// Upload one instance and draw.
    pub(crate) fn paint_instance(&mut self, ctx: PaintCtx<'_>, instance: &DecorationInstance) {
        let PaintCtx {
            device,
            queue,
            encoder,
            view,
            viewport,
        } = ctx;
        self.ensure_instance_capacity(device, 1);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&DecUniforms {
                viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
                _pad: [0.0, 0.0],
            }),
        );
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(instance));

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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(0..instance_stride()));
        pass.draw(0..6, 0..1);
    }
}

// ── Pipeline construction helpers ────────────────────────────────────────────

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

fn make_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil-renderer decoration uniforms"),
        size: core::mem::size_of::<DecUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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
            buffers: &[UNIT_QUAD_LAYOUT, INSTANCE_LAYOUT],
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

fn make_instance_buffer(device: &wgpu::Device, cap: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sigil-renderer decoration instance buffer"),
        size: cap * instance_stride(),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

// ── Instance builder ──────────────────────────────────────────────────────────

/// Convert a [`DrawDecoration`] command into a GPU instance record.
pub(crate) fn decoration_instance(draw: &DrawDecoration, opacity_scale: f32) -> DecorationInstance {
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
        color: [
            srgb_to_linear(draw.color.r),
            srgb_to_linear(draw.color.g),
            srgb_to_linear(draw.color.b),
            f32::from(draw.color.a) / 255.0,
        ],
        params: [opacity, draw.period, draw.line_width, kind_f32],
    }
}

fn srgb_to_linear(channel: u8) -> f32 {
    let v = f32::from(channel) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use pollster::FutureExt as _;

    use super::DecorationPipeline;

    #[test]
    fn decoration_pipeline_constructs_against_real_device() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
        else {
            eprintln!("no adapter — skipping decoration pipeline shader-validation test");
            return;
        };
        let Ok((device, _queue)) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("decoration-paint-test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .block_on()
        else {
            eprintln!("no device — skipping decoration pipeline shader-validation test");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let pipeline = DecorationPipeline::new(&device, format);
        assert_eq!(pipeline.format(), format);
    }
}
