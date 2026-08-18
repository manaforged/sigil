#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::redundant_pub_crate
)]
//! Dual-Kawase blur pipeline.
//!
//! [`BlurPipeline`] runs N downsample passes (`fs_down`) followed by N
//! upsample passes (`fs_up`), borrowing intermediate textures from the
//! renderer's [`LayerPool`]. The pass count is derived from `sigma` by
//! [`pass_count_for`].

use crate::blur_shader::BLUR_SHADER;
use crate::error::RendererError;
use crate::layer_pool::{LayerHandle, LayerPool};

mod setup;

use setup::{
    BlurInstance, BlurUniforms, UNIT_QUAD_VERTEX_COUNT, make_instance_buffer, make_pipeline,
    make_texture_bind_group, make_texture_layout, make_uniform_bind_group, make_uniform_buffer,
    make_uniform_layout, make_unit_quad,
};

/// Map a blur sigma (logical pixels) to the number of downsample/upsample passes.
///
/// The total render pass count is `pass_count * 2` (N down + N up).
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

/// Wgpu render pipeline for dual-Kawase blur.
///
/// Holds two pipelines that share a vertex shader and bind-group layout:
/// one for downsample (`fs_down`) and one for upsample (`fs_up`).
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

    /// Apply dual-Kawase blur to `src` and return the blurred result's view.
    ///
    /// Every intermediate texture (downsample pyramid + upsample outputs) is
    /// moved into `retained` so it outlives the recorded passes, which only run
    /// when the owner submits the encoder. Returns a clone of the final view
    /// (Arc-backed, so it keeps its texture alive) or `None` when `sigma == 0`
    /// or pool allocation fails.
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

    /// Build the N-level downsample pyramid. Returns the allocated handles.
    ///
    /// Each pass reads from the previous level's view. Because the pyramid `Vec`
    /// is the only owner of the handles, we access `pyramid[i-1].view` by index
    /// rather than keeping a dangling borrow across the `push`.
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
            // Borrow either the original src or the last pyramid level's view.
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

    /// Walk the pyramid in reverse, chaining each pass onto the previous
    /// upsample's output.
    ///
    /// The first upsample reads the smallest downsample level (`pyramid[n-1]`).
    /// Each subsequent pass reads the previous upsample's result. The final
    /// pass writes into a texture sized to `src_dims` (the original layer).
    fn upsample(
        &self,
        ctx: &mut PassCtx<'_>,
        pool: &mut LayerPool,
        pyramid: &[LayerHandle],
        src_dims: (u32, u32),
    ) -> Result<(LayerHandle, Vec<LayerHandle>), RendererError> {
        let (src_width, src_height) = src_dims;
        let n = pyramid.len();
        let seed = pyramid
            .last()
            .ok_or(RendererError::Layer(crate::layer_pool::LayerError::Empty))?;
        let mut intermediates: Vec<LayerHandle> = Vec::new();
        let mut current: Option<LayerHandle> = None;
        for up_pass in 0..n {
            let (target_w, target_h) = if up_pass + 1 == n {
                (src_width, src_height)
            } else {
                // Next-larger pyramid level. The first upsample reads
                // pyramid[n-1] and writes pyramid[n-2]-sized; the second reads
                // that output and writes pyramid[n-3]-sized; etc.
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
            .ok_or(RendererError::Layer(crate::layer_pool::LayerError::Empty))
    }

    fn run_pass(
        &self,
        ctx: &mut PassCtx<'_>,
        pipeline: &wgpu::RenderPipeline,
        src: PassSrc<'_>,
        dst: PassDst<'_>,
    ) {
        let (vw, vh) = dst.viewport;
        // Kawase passes read the full source level into the full destination
        // level — UV spans the unit quad. Fresh per-pass buffers + bind group
        // keep each recorded pass independent under one submission (DR-054).
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
        let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

/// Borrowed GPU context for a single blur pass.
pub(crate) struct PassCtx<'a> {
    pub(crate) device: &'a wgpu::Device,
    /// Shared encoder the blur passes record into. The layer walker owns the
    /// single submission, so blur output is sequenced with the surrounding
    /// clear / composite passes in one encoder (DR-054).
    pub(crate) encoder: &'a mut wgpu::CommandEncoder,
    /// Sink for the intermediate pyramid + upsample textures, kept alive until
    /// the owner submits `encoder` (the recorded passes reference them).
    pub(crate) retained: &'a mut Vec<LayerHandle>,
}

/// Source texture and logical dimensions for a single blur pass.
#[derive(Clone, Copy)]
pub(crate) struct PassSrc<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) w: u32,
    pub(crate) h: u32,
}

/// Destination texture view and viewport for a single blur pass.
#[derive(Clone, Copy)]
pub(crate) struct PassDst<'a> {
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) viewport: (f32, f32),
}
