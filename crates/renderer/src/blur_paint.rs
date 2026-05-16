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
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    unit_quad: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
}

impl BlurPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sigil-renderer blur shader"),
            source: wgpu::ShaderSource::Wgsl(BLUR_SHADER.into()),
        });
        let uniform_layout = make_uniform_layout(device);
        let texture_layout = make_texture_layout(device);
        let uniform_buffer = make_uniform_buffer(device);
        let uniform_bind_group = make_uniform_bind_group(device, &uniform_layout, &uniform_buffer);
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
            uniform_buffer,
            uniform_bind_group,
            texture_layout,
            sampler,
            unit_quad: make_unit_quad(device),
            instance_buffer: make_instance_buffer(device),
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Apply dual-Kawase blur to `src_view` and return the blurred result.
    ///
    /// Intermediate textures are borrowed from `pool`. Returns `None` when
    /// `sigma == 0` (skip) or pool allocation fails.
    pub(crate) fn apply(
        &self,
        ctx: PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        sigma: f32,
    ) -> Option<LayerHandle> {
        let passes = pass_count_for(sigma);
        if passes == 0 {
            return None;
        }
        let pyramid = self.downsample(ctx, pool, src, passes);
        if pyramid.is_empty() {
            return None;
        }
        self.upsample(ctx, pool, &pyramid, (src.w, src.h))
    }

    /// Build the N-level downsample pyramid. Returns the allocated handles.
    ///
    /// Each pass reads from the previous level's view. Because the pyramid `Vec`
    /// is the only owner of the handles, we access `pyramid[i-1].view` by index
    /// rather than keeping a dangling borrow across the `push`.
    fn downsample(
        &self,
        ctx: PassCtx<'_>,
        pool: &mut LayerPool,
        src: PassSrc<'_>,
        passes: u32,
    ) -> Vec<LayerHandle> {
        let mut pyramid: Vec<LayerHandle> = Vec::with_capacity(passes as usize);
        let mut w = src.w;
        let mut h = src.h;
        for _i in 0..passes {
            let next_w = (w / 2).max(1);
            let next_h = (h / 2).max(1);
            let Some(handle) = pool.acquire(ctx.device, next_w, next_h, self.format) else {
                break;
            };
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
        pyramid
    }

    /// Walk the pyramid in reverse, chaining each pass onto the previous
    /// upsample's output.
    ///
    /// The first upsample reads the smallest downsample level (`pyramid[n-1]`).
    /// Each subsequent pass reads the previous upsample's result. The final
    /// pass writes into a texture sized to `src_dims` (the original layer).
    fn upsample(
        &self,
        ctx: PassCtx<'_>,
        pool: &mut LayerPool,
        pyramid: &[LayerHandle],
        src_dims: (u32, u32),
    ) -> Option<LayerHandle> {
        let (src_width, src_height) = src_dims;
        let n = pyramid.len();
        let seed = pyramid.last()?;
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
            let Some(out_handle) = pool.acquire(ctx.device, target_w, target_h, self.format) else {
                break;
            };
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
            current = Some(out_handle);
        }
        current
    }

    fn run_pass(
        &self,
        ctx: PassCtx<'_>,
        pipeline: &wgpu::RenderPipeline,
        src: PassSrc<'_>,
        dst: PassDst<'_>,
    ) {
        self.upload_pass_state(ctx.queue, src, dst.viewport);
        let tex_bg =
            make_texture_bind_group(ctx.device, &self.texture_layout, src.view, &self.sampler);
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("sigil-renderer blur pass"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_bind_group(1, &tex_bg, &[]);
            pass.set_vertex_buffer(0, self.unit_quad.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..1);
        }
        ctx.queue.submit(core::iter::once(encoder.finish()));
    }

    fn upload_pass_state(&self, queue: &wgpu::Queue, src: PassSrc<'_>, viewport: (f32, f32)) {
        let _ = src;
        let dst_w = viewport.0.max(0.0) as u32;
        let dst_h = viewport.1.max(0.0) as u32;
        // Kawase passes always read the entire source pyramid level into the
        // entire destination level. UV is the unit quad → full source extent.
        // (`uv_max = dst/src` was copy-pasta from LayerCompositePipeline, where
        // it crops a rounded layer to its logical content region. That cropping
        // semantic does not apply inside the pyramid.)
        let instance = BlurInstance {
            dest: [0.0, 0.0, dst_w as f32, dst_h as f32],
            uv_max: [1.0, 1.0],
            opacity: 1.0,
            _pad: 0.0,
        };
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&BlurUniforms {
                viewport: [viewport.0.max(1.0), viewport.1.max(1.0)],
                _pad: [0.0, 0.0],
            }),
        );
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(&instance));
    }
}

/// Borrowed GPU context for a single blur pass.
#[derive(Clone, Copy)]
pub(crate) struct PassCtx<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
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
