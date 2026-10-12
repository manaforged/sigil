#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_pub_crate
)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use crate::renderer::blur_shader::BLUR_SHADER;
use crate::renderer::error::RendererError;
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, begin_pass, buffer_group, buffer_layout, linear_sampler, texture_group,
    texture_layout,
};
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::split_encoder::SplitEncoder;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurUniforms {
    viewport: [f32; 2],
    params: [f32; 2],
}

const _: () = assert!(core::mem::size_of::<BlurUniforms>() == 16);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BlurInstance {
    dest: [f32; 4],
    uv_max: [f32; 2],
    opacity: f32,
    _pad: f32,
}

const _: () = assert!(core::mem::size_of::<BlurInstance>() == 32);

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<BlurInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x2,
        3 => Float32,
    ],
};

pub(crate) const BLUR_PASSES: u32 = 3;

pub(crate) fn box_radius(sigma: f32) -> u32 {
    if sigma.is_nan() || sigma < 0.5 {
        return 0;
    }
    let width = (12.0 * sigma / BLUR_PASSES as f32)
        .mul_add(sigma, 1.0)
        .sqrt();
    ((width - 1.0) / 2.0).round() as u32
}

pub(crate) struct BlurPipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BlurPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = buffer_layout(
            device,
            "sigil-renderer blur uniforms layout",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            wgpu::BufferBindingType::Uniform,
        );
        let texture_layout = texture_layout(device, "sigil-renderer blur texture layout");
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer blur",
                wgsl: BLUR_SHADER,
                groups: &[Some(&uniform_layout), Some(&texture_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_box",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
            texture_layout,
            sampler: linear_sampler(device, "sigil-renderer blur sampler"),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn apply(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        sigma: f32,
    ) -> Result<Option<wgpu::TextureView>, RendererError> {
        let radius = box_radius(sigma);
        if radius == 0 {
            return Ok(None);
        }
        let pair = [
            pool.acquire(ctx.device, src.w, src.h, self.quad.format)
                .map_err(RendererError::Layer)?,
            pool.acquire(ctx.device, src.w, src.h, self.quad.format)
                .map_err(RendererError::Layer)?,
        ];
        let viewport = (src.w as f32, src.h as f32);
        for pass in 0..BLUR_PASSES * 2 {
            let horizontal = pass % 2 == 0;
            let (read, write) = if horizontal {
                (&pair[1], &pair[0])
            } else {
                (&pair[0], &pair[1])
            };
            let read_view = if pass == 0 { src.view } else { &read.view };
            self.run_pass(
                ctx,
                PassSrc {
                    view: read_view,
                    w: src.w,
                    h: src.h,
                },
                PassDst {
                    view: &write.view,
                    viewport,
                },
                [radius as f32, if horizontal { 1.0 } else { 0.0 }],
            );
        }
        let view = pair[1].view.clone();
        ctx.retained.extend(pair);
        Ok(Some(view))
    }

    fn run_pass(
        &self,
        ctx: &mut PassCtx<'_>,
        src: PassSrc<'_>,
        dst: PassDst<'_>,
        params: [f32; 2],
    ) {
        let (vw, vh) = dst.viewport;
        let instance = BlurInstance {
            dest: [0.0, 0.0, vw.max(0.0), vh.max(0.0)],
            uv_max: [1.0, 1.0],
            opacity: 1.0,
            _pad: 0.0,
        };
        let uniforms = BlurUniforms {
            viewport: [vw.max(1.0), vh.max(1.0)],
            params,
        };
        let uniform_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sigil-renderer blur uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let uniform_bg = buffer_group(
            ctx.device,
            "sigil-renderer blur uniforms bind group",
            &self.uniform_layout,
            uniform_buf.as_entire_buffer_binding(),
        );
        let instance_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sigil-renderer blur instance buffer"),
                contents: bytemuck::bytes_of(&instance),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let tex_bg = texture_group(
            ctx.device,
            "sigil-renderer blur texture bind group",
            &self.texture_layout,
            src.view,
            &self.sampler,
        );
        let load = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let pass = begin_pass(ctx.encoder, "sigil-renderer blur pass", dst.view, load);
        self.quad.draw(
            pass,
            &[&uniform_bg, &tex_bg],
            &[],
            instance_buf.slice(..),
            1,
        );
    }
}

pub(crate) struct PassCtx<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) encoder: &'a mut SplitEncoder,
    pub(crate) retained: &'a mut Vec<LayerHandle>,
}

#[derive(Clone, Copy)]
pub(crate) struct PassSrc<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) w: u32,
    pub(crate) h: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct PassDst<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) viewport: (f32, f32),
}
