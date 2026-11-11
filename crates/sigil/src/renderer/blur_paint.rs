#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_pub_crate
)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use crate::Error;
use crate::renderer::blur_shader::BLUR_SHADER;
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
    ((width - 1.0) / 2.0).round().min(MAX_BOX_RADIUS) as u32
}

const MAX_BOX_RADIUS: f32 = 1_048_576.0;
const MAX_BOX_TAPS: u32 = 256;
const MIN_SCALED_SIGMA: f32 = 16.0;
const MAX_DOWNSAMPLE_LEVELS: f32 = 15.0;
const ENCODE_IN: u32 = 2;
const DECODE_OUT: u32 = 4;

pub(crate) fn downsample_levels(sigma: f32) -> u32 {
    (sigma / MIN_SCALED_SIGMA)
        .log2()
        .floor()
        .clamp(0.0, MAX_DOWNSAMPLE_LEVELS) as u32
}

fn pass_flags(pass: u32, convert: bool) -> f32 {
    let horizontal = u32::from(pass.is_multiple_of(2));
    let encode_in = u32::from(convert && pass == 0) * ENCODE_IN;
    let decode_out = u32::from(convert && pass + 1 == BLUR_PASSES * 2) * DECODE_OUT;
    (horizontal | encode_in | decode_out | (MAX_BOX_TAPS << 3)) as f32
}

pub(crate) struct BlurPipeline {
    quad: QuadPipeline,
    down: QuadPipeline,
    up: QuadPipeline,
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
        let groups = [Some(&uniform_layout), Some(&texture_layout)];
        let quad = |label, fs_entry| {
            let desc = QuadDesc {
                label,
                wgsl: BLUR_SHADER,
                groups: &groups,
                instance: INSTANCE_LAYOUT,
                fs_entry,
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            };
            QuadPipeline::new(device, format, desc)
        };
        Self {
            quad: quad("sigil-renderer blur", "fs_box"),
            down: quad("sigil-renderer blur downsample", "fs_down"),
            up: quad("sigil-renderer blur upsample", "fs_up"),
            uniform_layout,
            texture_layout,
            sampler: linear_sampler(device, "sigil-renderer blur sampler"),
        }
    }

    pub(crate) fn apply(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        sigma: f32,
    ) -> Result<Option<wgpu::TextureView>, Error> {
        let levels = downsample_levels(sigma);
        let factor = 1_u32 << levels;
        let radius = box_radius(sigma / factor as f32);
        if radius == 0 {
            return Ok(None);
        }
        let mut small = (src.view.clone(), src.w, src.h);
        for level in 0..levels {
            let (w, h) = (small.1.div_ceil(2), small.2.div_ceil(2));
            let out = pool.acquire(ctx.device, w, h, self.quad.format)?;
            let flags = if level == 0 { ENCODE_IN } else { 0 };
            self.run_pass(
                &self.down,
                ctx,
                (&small.0, &out.view, (w as f32, h as f32)),
                [0.0, flags as f32],
            );
            small = (out.view.clone(), w, h);
            ctx.retained.push(out);
        }
        let blurred = self.box_passes(ctx, pool, &small, (radius, levels == 0))?;
        if levels == 0 {
            return Ok(Some(blurred));
        }
        let out = pool.acquire(ctx.device, src.w, src.h, self.quad.format)?;
        self.run_pass(
            &self.up,
            ctx,
            (&blurred, &out.view, (src.w as f32, src.h as f32)),
            [factor as f32, DECODE_OUT as f32],
        );
        let view = out.view.clone();
        ctx.retained.push(out);
        Ok(Some(view))
    }

    fn box_passes(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: &(wgpu::TextureView, u32, u32),
        (radius, convert): (u32, bool),
    ) -> Result<wgpu::TextureView, Error> {
        let (input, w, h) = src;
        let pair = [
            pool.acquire(ctx.device, *w, *h, self.quad.format)?,
            pool.acquire(ctx.device, *w, *h, self.quad.format)?,
        ];
        let viewport = (*w as f32, *h as f32);
        for pass in 0..BLUR_PASSES * 2 {
            let horizontal = pass % 2 == 0;
            let (read, write) = if horizontal {
                (&pair[1], &pair[0])
            } else {
                (&pair[0], &pair[1])
            };
            let read_view = if pass == 0 { input } else { &read.view };
            self.run_pass(
                &self.quad,
                ctx,
                (read_view, &write.view, viewport),
                [radius as f32, pass_flags(pass, convert)],
            );
        }
        let view = pair[1].view.clone();
        ctx.retained.extend(pair);
        Ok(view)
    }

    fn run_pass(
        &self,
        quad: &QuadPipeline,
        ctx: &mut PassCtx<'_>,
        (src, dst, (vw, vh)): (&wgpu::TextureView, &wgpu::TextureView, (f32, f32)),
        params: [f32; 2],
    ) {
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
        let uniform_bg = ctx.uniform_group(
            &self.uniform_layout,
            (
                "sigil-renderer blur uniforms",
                "sigil-renderer blur uniforms bind group",
            ),
            bytemuck::bytes_of(&uniforms),
        );
        let tex_bg = texture_group(
            ctx.device,
            "sigil-renderer blur texture bind group",
            &self.texture_layout,
            src,
            &self.sampler,
        );
        ctx.draw_once(
            quad,
            ("sigil-renderer blur pass", dst),
            &[&uniform_bg, &tex_bg],
            (
                "sigil-renderer blur instance buffer",
                bytemuck::bytes_of(&instance),
            ),
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

impl<'a> PassSrc<'a> {
    pub(crate) fn or_view<'b>(self, view: Option<&'b wgpu::TextureView>) -> PassSrc<'b>
    where
        'a: 'b,
    {
        PassSrc {
            view: view.unwrap_or(self.view),
            w: self.w,
            h: self.h,
        }
    }
}

impl PassCtx<'_> {
    pub(crate) fn uniform_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        (buffer_label, group_label): (&str, &str),
        contents: &[u8],
    ) -> wgpu::BindGroup {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(buffer_label),
                contents,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        buffer_group(
            self.device,
            group_label,
            layout,
            buffer.as_entire_buffer_binding(),
        )
    }

    pub(crate) fn draw_once(
        &mut self,
        quad: &QuadPipeline,
        (pass_label, target): (&'static str, &wgpu::TextureView),
        groups: &[&wgpu::BindGroup],
        (instance_label, instance): (&str, &[u8]),
    ) {
        let instance_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(instance_label),
                contents: instance,
                usage: wgpu::BufferUsages::VERTEX,
            });
        let load = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let pass = begin_pass(self.encoder, pass_label, target, load);
        quad.draw(pass, groups, &[], instance_buf.slice(..), 1);
    }
}
