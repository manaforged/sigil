#![allow(clippy::redundant_pub_crate)]
//! Scene paint — internal wgpu pipeline that paints `SceneItem::Rect`.
//!
//! This is the default renderer consumed by `WgpuRenderer::render_scene`.
//! It walks a [`Scene`] and instance-renders filled (optionally rounded)
//! rectangles. `SceneItem::Text` is painted separately by
//! [`crate::renderer::WgpuRenderer`] through the Slug text pipeline.
//!
//! Pipelines are cached per `wgpu::TextureFormat` inside the renderer's
//! `ReadyState` so one `WgpuRenderer` can serve surfaces with different
//! formats without rebuilding the pipeline per frame.
//!
//! See DR-017 (scene paint) and the `phase-08a-ergonomics-pass.md` plan.

use wgpu::util::DeviceExt as _;

use crate::scene::Scene;
use crate::scene_instance::{RectInstance, SceneUniforms, collect_instances, instance_stride};

/// Arguments to [`ScenePipeline::paint`]. Bundled to keep the argument
/// list under the clippy `too_many_arguments` threshold.
pub(crate) struct PaintCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub viewport: (f32, f32),
}

/// Wgpu resources for painting `SceneItem::Rect`. One per surface format.
pub(crate) struct ScenePipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    unit_quad: wgpu::Buffer,
    instance_cap: u64,
    instance_buffer: wgpu::Buffer,
}

impl ScenePipeline {
    #[allow(clippy::too_many_lines)]
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sigil-renderer scene uniforms"),
            size: u64::try_from(core::mem::size_of::<SceneUniforms>()).unwrap_or(0),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sigil-renderer scene uniforms bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
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
                buffers: &[UNIT_QUAD_LAYOUT, INSTANCE_LAYOUT],
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

        let initial_cap = 64_u64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sigil-renderer scene instance buffer"),
            size: initial_cap * instance_stride(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
        let mut new_cap = self.instance_cap.max(1);
        while new_cap < required {
            new_cap = new_cap.saturating_mul(2);
        }
        self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sigil-renderer scene instance buffer"),
            size: new_cap * instance_stride(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_cap = new_cap;
    }

    /// Upload instances + draw.
    ///
    /// `viewport` is the logical-pixel size matching the surface
    /// coordinate system the scene rects were laid out in.
    #[allow(dead_code)]
    pub(crate) fn paint(&mut self, ctx: PaintCtx<'_>, scene: &Scene) {
        let instances = collect_instances(scene);
        self.paint_instances(ctx, &instances);
    }

    pub(crate) fn paint_instances(&mut self, ctx: PaintCtx<'_>, instances: &[RectInstance]) {
        let PaintCtx {
            device,
            queue,
            encoder,
            view,
            viewport,
        } = ctx;
        if instances.is_empty() {
            return;
        }

        let instance_count = u32::try_from(instances.len()).unwrap_or(u32::MAX);
        self.ensure_instance_capacity(device, instances.len() as u64);

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&SceneUniforms {
                viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
                _pad: [0.0, 0.0],
            }),
        );
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(
            1,
            self.instance_buffer
                .slice(0..instance_stride() * u64::from(instance_count)),
        );
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instance_count);
    }
}

const UNIT_QUAD_VERTEX_COUNT: u32 = 6;

/// Two triangles covering `[0, 0]`..`[1, 1]`.
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
        1 => Float32x4,  // rect
        2 => Float32x4,  // color
        3 => Float32x4,  // params + pad
    ],
};

/// Scene paint rectangle shader.
///
/// Takes per-vertex unit-quad positions and per-instance `(rect, color,
/// params)`. Converts logical pixels to NDC using the viewport uniform.
/// Fragment shader applies a rounded-rect SDF for corner anti-aliasing
/// when `radius > 0`.
const RECT_SHADER: &str = "\
struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    @location(1) rect: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) params: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) radius: f32,
    @location(4) opacity: f32,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pos_px = inst.rect.xy + v.unit * inst.rect.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.color = inst.color;
    let half = inst.rect.zw * 0.5;
    out.local = v.unit * inst.rect.zw - half;
    out.half_size = half;
    out.radius = inst.params.x;
    out.opacity = inst.params.y;
    return out;
}

fn rounded_rect_sdf(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var alpha = 1.0;
    if in.radius > 0.0 {
        let d = rounded_rect_sdf(in.local, in.half_size, in.radius);
        alpha = 1.0 - smoothstep(-0.5, 0.5, d);
    }
    return vec4<f32>(in.color.rgb, in.color.a * in.opacity * alpha);
}
";

/// Emit a one-per-frame log if the scene contains `SceneItem::Text`.
/// Text paint is Phase 14 (text-in-scene + IME).
#[cfg(test)]
mod tests {
    use pollster::FutureExt as _;

    use super::ScenePipeline;

    /// Constructs the scene pipeline against a real GPU adapter so the
    /// shader is parsed + validated. Catches WGSL keyword collisions
    /// (the kind of bug that crashed `hello-style` on launch with
    /// `name 'meta' is a reserved keyword`).
    #[test]
    fn scene_pipeline_constructs_against_real_device() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
        else {
            eprintln!("no adapter — skipping scene-pipeline shader-validation test");
            return;
        };
        let Ok((device, _queue)) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("scene-paint-test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .block_on()
        else {
            eprintln!("no device — skipping scene-pipeline shader-validation test");
            return;
        };
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let pipeline = ScenePipeline::new(&device, format);
        assert_eq!(pipeline.format(), format);
    }
}
