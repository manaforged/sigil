#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::redundant_pub_crate
)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt as _;

use crate::Error;
use crate::renderer::blur_paint::{BlurPipeline, PassCtx, PassSrc};
use crate::renderer::fill_paint::setup::{
    QuadDesc, QuadPipeline, begin_pass, buffer_group, buffer_layout, linear_sampler, texture_group,
    texture_layout,
};
use crate::renderer::layer_pool::LayerPool;
use crate::scene::command::Filter;
use crate::scene::shadow::Shadow;

pub(crate) mod chain;
pub(crate) mod matrix;
mod shader;

pub(crate) use chain::LayerFilters;
use matrix::color_matrix;
use shader::FILTER_SHADER;

const MAX_MATRICES: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FilterUniforms {
    viewport: [f32; 2],
    offset: [f32; 2],
    tint: [f32; 4],
    count: [u32; 4],
    rows: [[f32; 4]; MAX_MATRICES * 5],
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<[f32; 4]>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![1 => Float32x4],
};

pub(crate) struct FilterPipeline {
    blur: BlurPipeline,
    matrix: QuadPipeline,
    shadow: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

struct FilterPass<'a> {
    src: PassSrc<'a>,
    shadow: &'a wgpu::TextureView,
    uniforms: FilterUniforms,
}

impl FilterPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = buffer_layout(
            device,
            "sigil-renderer filter uniforms layout",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            wgpu::BufferBindingType::Uniform,
        );
        let texture_layout = texture_layout(device, "sigil-renderer filter texture layout");
        let groups = [
            Some(&uniform_layout),
            Some(&texture_layout),
            Some(&texture_layout),
        ];
        let quad = |label, fs_entry| {
            let desc = QuadDesc {
                label,
                wgsl: FILTER_SHADER,
                groups: &groups,
                instance: INSTANCE_LAYOUT,
                fs_entry,
                blend: wgpu::BlendState::REPLACE,
            };
            QuadPipeline::new(device, format, desc)
        };
        Self {
            blur: BlurPipeline::new(device, format),
            matrix: quad("sigil-renderer filter matrix", "fs_matrix"),
            shadow: quad("sigil-renderer filter shadow", "fs_shadow"),
            uniform_layout,
            texture_layout,
            sampler: linear_sampler(device, "sigil-renderer filter sampler"),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.matrix.format
    }

    pub(crate) fn apply(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        (filters, scale): (&LayerFilters, f32),
    ) -> Result<Option<wgpu::TextureView>, Error> {
        let mut current: Option<wgpu::TextureView> = None;
        for step in filters.steps() {
            let input = PassSrc {
                view: current.as_ref().unwrap_or(src.view),
                ..src
            };
            let next = match *step {
                [Filter::Blur(sigma)] => self.blur.apply(ctx, pool, input, sigma * scale)?,
                [Filter::DropShadow(shadow)] => {
                    Some(self.drop_shadow(ctx, pool, input, (shadow, scale))?)
                }
                ref colors => self.colors(ctx, pool, input, colors)?,
            };
            if let Some(view) = next {
                current = Some(view);
            }
        }
        Ok(current)
    }

    fn colors(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        colors: &[Filter],
    ) -> Result<Option<wgpu::TextureView>, Error> {
        let mut current: Option<wgpu::TextureView> = None;
        for run in colors.chunks(MAX_MATRICES) {
            let mut uniforms = uniforms(src);
            let matrices = run.iter().filter_map(|f| color_matrix(*f));
            for (block, matrix) in uniforms
                .rows
                .as_chunks_mut::<5>()
                .0
                .iter_mut()
                .zip(matrices)
            {
                uniforms.count[0] += 1;
                let [rows @ .., offsets] = block;
                for ((slot, offset), [weights @ .., bias]) in
                    rows.iter_mut().zip(offsets.iter_mut()).zip(matrix)
                {
                    *slot = weights;
                    *offset = bias;
                }
            }
            let input = PassSrc {
                view: current.as_ref().unwrap_or(src.view),
                ..src
            };
            let pass = FilterPass {
                src: input,
                shadow: input.view,
                uniforms,
            };
            current = Some(self.run(ctx, pool, &self.matrix, &pass)?);
        }
        Ok(current)
    }

    fn drop_shadow(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        (shadow, scale): (Shadow, f32),
    ) -> Result<wgpu::TextureView, Error> {
        let blurred = self.blur.apply(ctx, pool, src, shadow.blur * scale)?;
        let [r, g, b, a] = shadow.color;
        let mut uniforms = uniforms(src);
        uniforms.offset = shadow.offset.map(|value| (value * scale).round());
        uniforms.tint = [r * a, g * a, b * a, a];
        let pass = FilterPass {
            src,
            shadow: blurred.as_ref().unwrap_or(src.view),
            uniforms,
        };
        self.run(ctx, pool, &self.shadow, &pass)
    }

    fn run(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        quad: &QuadPipeline,
        pass: &FilterPass<'_>,
    ) -> Result<wgpu::TextureView, Error> {
        let (w, h) = (pass.src.w, pass.src.h);
        let out = pool.acquire(ctx.device, w, h, quad.format)?;
        let groups = self.groups(ctx.device, pass);
        let instance_buf = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sigil-renderer filter instance"),
                contents: bytemuck::bytes_of(&[0.0, 0.0, w as f32, h as f32]),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let load = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let render = begin_pass(ctx.encoder, "sigil-renderer filter pass", &out.view, load);
        quad.draw(
            render,
            &[&groups[0], &groups[1], &groups[2]],
            &[],
            instance_buf.slice(..),
            1,
        );
        let view = out.view.clone();
        ctx.retained.push(out);
        Ok(view)
    }

    fn groups(&self, device: &wgpu::Device, pass: &FilterPass<'_>) -> [wgpu::BindGroup; 3] {
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sigil-renderer filter uniforms"),
            contents: bytemuck::bytes_of(&pass.uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        [
            buffer_group(
                device,
                "sigil-renderer filter uniforms bind group",
                &self.uniform_layout,
                uniform_buf.as_entire_buffer_binding(),
            ),
            self.texture(device, pass.src.view),
            self.texture(device, pass.shadow),
        ]
    }

    fn texture(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        texture_group(
            device,
            "sigil-renderer filter texture bind group",
            &self.texture_layout,
            view,
            &self.sampler,
        )
    }
}

fn uniforms(src: PassSrc<'_>) -> FilterUniforms {
    FilterUniforms {
        viewport: [src.w.max(1) as f32, src.h.max(1) as f32],
        offset: [0.0; 2],
        tint: [0.0; 4],
        count: [0; 4],
        rows: [[0.0; 4]; MAX_MATRICES * 5],
    }
}
