#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_pub_crate
)]

use crate::renderer::blur_shader::BLUR_SHADER;
use crate::renderer::error::RendererError;
use crate::renderer::layer_pool::{LayerHandle, LayerPool};
use crate::renderer::split_encoder::SplitEncoder;

mod setup;

use setup::{
    BlurInstance, BlurUniforms, UNIT_QUAD_VERTEX_COUNT, make_instance_buffer, make_pipeline,
    make_texture_bind_group, make_texture_layout, make_uniform_bind_group, make_uniform_buffer,
    make_uniform_layout, make_unit_quad,
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
    format: wgpu::TextureFormat,
    box_pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    unit_quad: wgpu::Buffer,
}

impl BlurPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sigil-renderer blur shader"),
            source: wgpu::ShaderSource::Wgsl(BLUR_SHADER.into()),
        });
        let uniform_layout = make_uniform_layout(device);
        let texture_layout = make_texture_layout(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sigil-renderer blur sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sigil-renderer blur pipeline layout"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let box_pipeline = make_pipeline(device, format, &shader, &layout, "fs_box");
        Self {
            format,
            box_pipeline,
            uniform_layout,
            texture_layout,
            sampler,
            unit_quad: make_unit_quad(device),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
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
            pool.acquire(ctx.device, src.w, src.h, self.format)
                .map_err(RendererError::Layer)?,
            pool.acquire(ctx.device, src.w, src.h, self.format)
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
        let uniform_buf = make_uniform_buffer(
            ctx.device,
            &BlurUniforms {
                viewport: [vw.max(1.0), vh.max(1.0)],
                params,
            },
        );
        let uniform_bg = make_uniform_bind_group(ctx.device, &self.uniform_layout, &uniform_buf);
        let instance_buf = make_instance_buffer(ctx.device, &instance);
        let tex_bg =
            make_texture_bind_group(ctx.device, &self.texture_layout, src.view, &self.sampler);
        let pass = ctx.encoder.begin_render_pass(
            "sigil-renderer blur pass",
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            },
        );
        pass.set_pipeline(&self.box_pipeline);
        pass.set_bind_group(0, &uniform_bg, &[]);
        pass.set_bind_group(1, &tex_bg, &[]);
        pass.set_vertex_buffer(0, self.unit_quad.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..1);
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
