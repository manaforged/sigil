#![allow(clippy::cast_precision_loss)]

use bytemuck::{Pod, Zeroable};
use std::slice::from_ref;

#[path = "layer_composite/shader.rs"]
mod shader;
#[path = "layer_composite/variant.rs"]
mod variant;

use crate::geometry::Rect;
use crate::profile::{count, span};
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, buffer_group, clipped_pass, linear_sampler, texture_group,
    texture_layout,
};
use crate::renderer::scene_instance::SceneUniforms;
use crate::renderer::scene_paint::PaintCtx;
use crate::scene::command::{BlendMode, MaskMode};
use crate::scene::{CornerRadii, Transform};
use shader::LAYER_SHADER;
use variant::{BLENDED, MASKED, Variant, blend_index};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct LayerInstance {
    dest: [f32; 4],
    clip_radii: [f32; 4],
    uv_max: [f32; 2],
    opacity: f32,
    transform_x: [f32; 3],
    transform_y: [f32; 3],
    round: [f32; 4],
    mask: [f32; 4],
    luminance: f32,
    backdrop: [f32; 2],
    blend: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct Backdrop<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) origin: [f32; 2],
    pub(crate) mode: BlendMode,
}

#[derive(Clone, Copy, Default)]
struct Inputs<'a> {
    mask: Option<LayerMask<'a>>,
    backdrop: Option<Backdrop<'a>>,
}

#[derive(Clone, Copy)]
pub(crate) struct LayerMask<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) rect: [f32; 4],
    pub(crate) mode: MaskMode,
}

pub(crate) struct LayerCompositePipeline {
    quad: QuadPipeline,
    masked: Option<QuadPipeline>,
    blended: Option<QuadPipeline>,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: Option<(wgpu::Buffer, wgpu::BindGroup)>,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    sampler: wgpu::Sampler,
}

impl LayerCompositePipeline {
    pub(crate) fn for_format<'a>(
        pipelines: &'a mut Vec<Self>,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> &'a mut Self {
        let idx = pipelines
            .iter()
            .position(|p| p.format() == format)
            .unwrap_or_else(|| {
                let _span = span(crate::profile::Phase::RenderCompositePipeline);
                count(crate::profile::Counter::GpuCompositePipelines);
                pipelines.push(Self::new(device, format));
                pipelines.len() - 1
            });
        pipelines
            .get_mut(idx)
            .expect("layer pipeline index was just constructed from the vec")
    }

    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = uniform_layout(device, "sigil-renderer layer uniforms layout");
        let texture_layout = texture_layout(device, "sigil-renderer layer texture layout");
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer layer composite",
                wgsl: LAYER_SHADER,
                groups: &[Some(&uniform_layout), Some(&texture_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            masked: None,
            blended: None,
            uniform_layout,
            uniforms: None,
            texture_layout,
            textures: Vec::new(),
            sampler: linear_sampler(device, "sigil-renderer layer composite sampler"),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn retain(&mut self, keep: impl Fn(&wgpu::TextureView) -> bool) {
        self.textures.retain(|(view, _)| keep(view));
    }

    pub(crate) fn composite(&mut self, ctx: PaintCtx<'_>, request: CompositeRequest<'_>) {
        self.draw(ctx, request, Inputs::default());
    }

    pub(crate) fn composite_masked(
        &mut self,
        ctx: PaintCtx<'_>,
        request: CompositeRequest<'_>,
        mask: LayerMask<'_>,
    ) {
        if self.masked.is_none() {
            self.masked = Some(self.extra_pipeline(ctx.device, MASKED));
        }
        let inputs = Inputs {
            mask: Some(mask),
            backdrop: None,
        };
        self.draw(ctx, request, inputs);
    }

    pub(crate) fn composite_blended(
        &mut self,
        ctx: PaintCtx<'_>,
        request: CompositeRequest<'_>,
        mask: Option<LayerMask<'_>>,
        backdrop: Backdrop<'_>,
    ) {
        if self.blended.is_none() {
            self.blended = Some(self.extra_pipeline(ctx.device, BLENDED));
        }
        let inputs = Inputs {
            mask,
            backdrop: Some(backdrop),
        };
        self.draw(ctx, request, inputs);
    }

    fn draw(&mut self, mut ctx: PaintCtx<'_>, request: CompositeRequest<'_>, inputs: Inputs<'_>) {
        let instance = instance_with(&request, inputs);
        let offset = self.uniform(&mut ctx);
        let instances = ctx.encoder.upload(from_ref(&instance));
        let texture = self.texture(ctx.device, request.layer_view);
        let masked = inputs.mask.map(|mask| self.texture(ctx.device, mask.view));
        let behind = inputs
            .backdrop
            .map(|backdrop| self.texture(ctx.device, backdrop.view));
        let (_, uniform_bg) = self.uniforms.as_ref().expect("layer uniforms initialized");
        let group = |index: usize| {
            self.textures
                .get(index)
                .map(|(_, group)| group)
                .expect("layer texture initialized")
        };
        let base = group(texture);
        let all = [
            uniform_bg,
            base,
            masked.map_or(base, group),
            behind.map_or(base, group),
        ];
        let (quad, count) = match (behind, masked) {
            (Some(_), _) => (self.blended.as_ref(), 4),
            (None, Some(_)) => (self.masked.as_ref(), 3),
            (None, None) => (Some(&self.quad), 2),
        };
        let Some(quad) = quad else {
            return;
        };
        if let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer layer composite pass") {
            quad.draw(
                pass,
                all.get(..count).unwrap_or_default(),
                &[offset],
                instances.buffer.slice(instances.bytes),
                1,
            );
        }
    }

    fn extra_pipeline(&self, device: &wgpu::Device, variant: Variant) -> QuadPipeline {
        let texture = Some(&self.texture_layout);
        let groups = [Some(&self.uniform_layout), texture, texture, texture];
        QuadPipeline::new(
            device,
            self.quad.format,
            QuadDesc {
                label: variant.label,
                wgsl: LAYER_SHADER,
                groups: groups.get(..variant.groups).unwrap_or_default(),
                instance: INSTANCE_LAYOUT,
                fs_entry: variant.fs_entry,
                blend: variant.blend,
            },
        )
    }

    fn uniform(&mut self, ctx: &mut PaintCtx<'_>) -> u32 {
        viewport_uniform(
            ctx,
            &self.uniform_layout,
            "sigil-renderer layer uniforms bind group",
            &mut self.uniforms,
        )
    }

    fn texture(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) -> usize {
        self.textures
            .iter()
            .position(|(cached, _)| cached == view)
            .unwrap_or_else(|| {
                count(crate::profile::Counter::AllocCompositeTextureBindGroups);
                let group = texture_group(
                    device,
                    "sigil-renderer layer texture bind group",
                    &self.texture_layout,
                    view,
                    &self.sampler,
                );
                self.textures.push((view.clone(), group));
                self.textures.len() - 1
            })
    }
}

fn instance_with(request: &CompositeRequest<'_>, inputs: Inputs<'_>) -> LayerInstance {
    let mut instance = instance_of(request);
    if let Some(mask) = inputs.mask {
        instance.mask = mask.rect;
        instance.luminance = match mask.mode {
            MaskMode::Luminance => 1.0,
            MaskMode::Alpha => 0.0,
        };
    }
    if let Some(backdrop) = inputs.backdrop {
        instance.backdrop = backdrop.origin;
        instance.blend = blend_index(backdrop.mode);
    }
    instance
}

fn instance_of(request: &CompositeRequest<'_>) -> LayerInstance {
    LayerInstance {
        dest: [
            request.dest.origin.x,
            request.dest.origin.y,
            request.dest.width,
            request.dest.height,
        ],
        clip_radii: request
            .clip_radii
            .normalized_for(request.round.width, request.round.height),
        uv_max: uv_max(request.source_extent, request.texture_size),
        opacity: request.opacity.clamp(0.0, 1.0),
        transform_x: request.transform[0],
        transform_y: request.transform[1],
        round: [
            request.round.width.mul_add(0.5, request.round.origin.x),
            request.round.height.mul_add(0.5, request.round.origin.y),
            request.round.width * 0.5,
            request.round.height * 0.5,
        ],
        mask: [0.0; 4],
        luminance: 0.0,
        backdrop: [0.0; 2],
        blend: 0,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CompositeRequest<'a> {
    pub(crate) layer_view: &'a wgpu::TextureView,
    pub(crate) dest: Rect,
    pub(crate) round: Rect,
    pub(crate) texture_size: (u32, u32),
    pub(crate) source_extent: (u32, u32),
    pub(crate) opacity: f32,
    pub(crate) clip_radii: CornerRadii,
    pub(crate) transform: Transform,
}

pub(crate) fn uv_max(source_extent: (u32, u32), texture_size: (u32, u32)) -> [f32; 2] {
    [
        source_extent.0 as f32 / texture_size.0.max(1) as f32,
        source_extent.1 as f32 / texture_size.1.max(1) as f32,
    ]
}

pub(crate) fn uniform_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(size_of::<SceneUniforms>() as u64),
            },
            count: None,
        }],
    })
}

pub(crate) fn viewport_uniform(
    ctx: &mut PaintCtx<'_>,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    cache: &mut Option<(wgpu::Buffer, wgpu::BindGroup)>,
) -> u32 {
    let (buffer, offset, _) = ctx.encoder.uniform(&SceneUniforms {
        viewport: [ctx.viewport.0.max(1.0), ctx.viewport.1.max(1.0)],
        _pad: [0.0, 0.0],
    });
    if cache.as_ref().is_none_or(|(cached, _)| *cached != buffer) {
        count(crate::profile::Counter::AllocCompositeUniformBindGroups);
        let binding = wgpu::BufferBinding {
            buffer: &buffer,
            offset: 0,
            size: wgpu::BufferSize::new(size_of::<SceneUniforms>() as u64),
        };
        let group = buffer_group(ctx.device, label, layout, binding);
        *cache = Some((buffer, group));
    }
    offset
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<LayerInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x2,
        4 => Float32,
        5 => Float32x3,
        6 => Float32x3,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32,
        10 => Float32x2,
        11 => Uint32,
    ],
};
