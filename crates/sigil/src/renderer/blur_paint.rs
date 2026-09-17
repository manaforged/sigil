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

pub(crate) fn pass_count_for(sigma: f32) -> u32 {
    if sigma <= 0.0 {
        0
    } else if sigma <= 8.0 {
        1
    } else if sigma <= 16.0 {
        2
    } else {
        3
    }
}

pub(crate) struct BlurPipeline {
    format: wgpu::TextureFormat,
    down_pipeline: wgpu::RenderPipeline,
    up_pipeline: wgpu::RenderPipeline,
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
        let down_pipeline = make_pipeline(device, format, &shader, &layout, "fs_down");
        let up_pipeline = make_pipeline(device, format, &shader, &layout, "fs_up");
        Self {
            format,
            down_pipeline,
            up_pipeline,
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
        let passes = pass_count_for(sigma);
        if passes == 0 {
            return Ok(None);
        }
        let pyramid = self.downsample(ctx, pool, src, passes)?;
        let (result, intermediates) = self.upsample(ctx, pool, &pyramid, (src.w, src.h))?;
        let view = result.view.clone();
        ctx.retained.extend(pyramid);
        ctx.retained.extend(intermediates);
        ctx.retained.push(result);
        Ok(Some(view))
    }

    fn downsample(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        passes: u32,
    ) -> Result<Vec<LayerHandle>, RendererError> {
        let mut pyramid: Vec<LayerHandle> = Vec::with_capacity(passes as usize);
        let mut w = src.w;
        let mut h = src.h;
        for _i in 0..passes {
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);
            let handle = pool
                .acquire(ctx.device, next_w, next_h, self.format)
                .map_err(RendererError::Layer)?;
            let vp = (handle.width as f32, handle.height as f32);
            let read_view: &wgpu::TextureView =
                pyramid.last().map_or(src.view, |lv: &LayerHandle| &lv.view);
            self.run_pass(
                ctx,
                &self.down_pipeline,
                PassSrc {
                    view: read_view,
                    w,
                    h,
                },
                PassDst {
                    view: &handle.view,
                    viewport: vp,
                },
            );
            w = handle.width;
            h = handle.height;
            pyramid.push(handle);
        }
        Ok(pyramid)
    }

    fn upsample(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        pyramid: &[LayerHandle],
        src_dims: (u32, u32),
    ) -> Result<(LayerHandle, Vec<LayerHandle>), RendererError> {
        let (src_width, src_height) = src_dims;
        let n = pyramid.len();
        let seed = pyramid.last().ok_or(RendererError::Layer(
            crate::renderer::layer_pool::LayerError::Empty,
        ))?;
        let mut intermediates: Vec<LayerHandle> = Vec::new();
        let mut current: Option<LayerHandle> = None;
        for up_pass in 0..n {
            let (target_w, target_h) = if up_pass + 1 == n {
                (src_width, src_height)
            } else {
                pyramid
                    .get(n - up_pass - 2)
                    .map_or((src_width, src_height), |lv| (lv.width, lv.height))
            };
            let out_handle = pool
                .acquire(ctx.device, target_w, target_h, self.format)
                .map_err(RendererError::Layer)?;
            let vp = (out_handle.width as f32, out_handle.height as f32);
            {
                let src_handle: &LayerHandle = current.as_ref().unwrap_or(seed);
                self.run_pass(
                    ctx,
                    &self.up_pipeline,
                    PassSrc {
                        view: &src_handle.view,
                        w: src_handle.width,
                        h: src_handle.height,
                    },
                    PassDst {
                        view: &out_handle.view,
                        viewport: vp,
                    },
                );
            }
            if let Some(prev) = current.take() {
                intermediates.push(prev);
            }
            current = Some(out_handle);
        }
        current
            .map(|result| (result, intermediates))
            .ok_or(RendererError::Layer(
                crate::renderer::layer_pool::LayerError::Empty,
            ))
    }

    fn run_pass(
        &self,
        ctx: &mut PassCtx<'_>,
        pipeline: &wgpu::RenderPipeline,
        src: PassSrc<'_>,
        dst: PassDst<'_>,
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
                _pad: [0.0, 0.0],
            },
        );
        let uniform_bg = make_uniform_bind_group(ctx.device, &self.uniform_layout, &uniform_buf);
        let instance_buf = make_instance_buffer(ctx.device, &instance);
        let tex_bg =
            make_texture_bind_group(ctx.device, &self.texture_layout, src.view, &self.sampler);
        let pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sigil-renderer blur pass"),
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
        });
        pass.set_pipeline(pipeline);
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
