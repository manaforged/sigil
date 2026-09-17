use super::color::polarity_flag;
use std::collections::HashMap;

use crate::hal::{Color, Rect, RenderedGlyph};
use crate::text::Paragraph;
use wgpu::util::DeviceExt as _;

use crate::renderer::error::RendererError;
use crate::renderer::frame_stats::FrameStats;
use crate::renderer::frame_upload::FrameUploads;
use crate::renderer::slug::{
    atlas::GlyphAtlas,
    bands::preprocess_glyph,
    cache::{GlyphCache, GlyphKey},
    pipeline::{create_bind_group_layout, create_pipeline},
};
use crate::renderer::split_encoder::SplitEncoder;

pub struct DrawParams<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub surface_width: f32,
    pub surface_height: f32,
    pub scale: f32,
    pub paragraph: &'a Paragraph,
    pub glyphs: Option<&'a [RenderedGlyph]>,
    pub origin: (f32, f32),
    pub color: Option<Color>,
    pub clip: Option<Rect>,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct SurfaceUniform {
    surface_size: [f32; 2],
    scale: f32,
    dilation: f32,
    sharpen: f32,
    gamma_lod: f32,
    gamma_dol: f32,
    enabled: f32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub(crate) struct GlyphInstance {
    origin: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    atlas_offset: u32,
    polarity: f32,
}
pub(crate) const SLUG_INSTANCE_BYTES: usize = core::mem::size_of::<GlyphInstance>();

pub(crate) struct QueuedRun {
    pub(crate) instances: Vec<GlyphInstance>,
    pub(crate) clip: Option<Rect>,
}

pub(crate) struct RunFrame<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) surface_width: f32,
    pub(crate) surface_height: f32,
    pub(crate) scale: f32,
}

const QUAD_UVS: &[f32] = &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];

pub struct SlugTextPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    cache: GlyphCache,
    quad_vb: wgpu::Buffer,
    atlas: GlyphAtlas,
}

impl SlugTextPipeline {
    #[must_use]
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let bgl = create_bind_group_layout(device);
        let pipeline = create_pipeline(device, &bgl, surface_format);
        let quad_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slug-quad-vb"),
            contents: bytemuck::cast_slice(QUAD_UVS),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let atlas = GlyphAtlas::new(device);
        Self {
            format: surface_format,
            pipeline,
            bgl,
            cache: GlyphCache::new(),
            quad_vb,
            atlas,
        }
    }

    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn draw_paragraph(&mut self, params: &DrawParams<'_>) -> Result<(), RendererError> {
        let encoder = params
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("slug-encoder"),
            });
        let mut encoder = SplitEncoder::new(params.device, params.queue, encoder);
        self.record_paragraph_in_frame(&mut encoder, params, None);
        params
            .queue
            .submit(core::iter::once(encoder.into_inner().finish()));
        Ok(())
    }

    pub fn record_paragraph(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        params: &DrawParams<'_>,
    ) -> Result<(), RendererError> {
        let real = std::mem::replace(
            encoder,
            params
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("slug-encoder placeholder"),
                }),
        );
        let mut split = SplitEncoder::new(params.device, params.queue, real);
        self.record_paragraph_in_frame(&mut split, params, None);
        *encoder = split.into_inner();
        Ok(())
    }

    pub(crate) fn record_paragraph_in_frame(
        &mut self,
        encoder: &mut SplitEncoder,
        params: &DrawParams<'_>,
        frame: Option<&mut FrameStats>,
    ) {
        let owned_glyphs;
        let glyphs = if let Some(glyphs) = params.glyphs {
            glyphs
        } else {
            owned_glyphs = params.paragraph.rendered_glyphs();
            &owned_glyphs
        };
        let instances = self.instances_for(glyphs, params);
        let run = QueuedRun {
            instances,
            clip: params.clip,
        };
        let frame_params = RunFrame {
            device: params.device,
            view: params.view,
            surface_width: params.surface_width,
            surface_height: params.surface_height,
            scale: params.scale,
        };
        self.record_runs_in_frame(encoder, &frame_params, &[run], frame);
    }

    pub(crate) fn instances_for(
        &mut self,
        glyphs: &[RenderedGlyph],
        params: &DrawParams<'_>,
    ) -> Vec<GlyphInstance> {
        if glyphs.is_empty() {
            return Vec::new();
        }
        self.ensure_preprocessed(glyphs, params.paragraph);
        self.atlas.rebuild(params.device, params.queue, &self.cache);
        self.build_instances(glyphs, params)
    }

    pub(crate) fn record_runs_in_frame(
        &self,
        encoder: &mut SplitEncoder,
        frame: &RunFrame<'_>,
        runs: &[QueuedRun],
        stats: Option<&mut FrameStats>,
    ) {
        let all: Vec<GlyphInstance> = runs
            .iter()
            .flat_map(|run| run.instances.iter().copied())
            .collect();
        if all.is_empty() {
            return;
        }
        let uploaded = stats.and_then(|stats| FrameUploads::upload(encoder, &all, stats));
        let range = uploaded.unwrap_or_else(|| encoder.upload(&all));
        let bind_group = make_bind_group(encoder, frame, &self.bgl, self.atlas.buffer());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("slug-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame.view,
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, range.buffer.slice(range.bytes));
        let viewport = (frame.surface_width, frame.surface_height);
        let mut first = 0_u32;
        for run in runs {
            let count = u32::try_from(run.instances.len()).unwrap_or(u32::MAX);
            let slice = first..first.saturating_add(count);
            first = slice.end;
            if count == 0 || !crate::renderer::scissor::apply(pass, run.clip, frame.scale, viewport)
            {
                continue;
            }
            pass.draw(0..6, slice);
        }
    }

    fn ensure_preprocessed(&mut self, glyphs: &[RenderedGlyph], para: &Paragraph) {
        let mut missing: HashMap<(u64, u32), Vec<usize>> = HashMap::new();
        for (i, g) in glyphs.iter().enumerate() {
            let key = GlyphKey {
                font_id: g.font_id,
                font_index: g.font_index,
                glyph_id: g.glyph_id,
                variation_coords: g.variation_coords.clone(),
            };
            if !self.cache.contains(&key) {
                missing
                    .entry((g.font_id, g.font_index))
                    .or_default()
                    .push(i);
            }
        }
        if missing.is_empty() {
            return;
        }
        let cache = &mut self.cache;
        para.for_each_font(|font_id, bytes, font_index| {
            let Some(indices) = missing.get(&(font_id, font_index)) else {
                return;
            };
            for &i in indices {
                let Some(g) = glyphs.get(i) else { continue };
                let key = GlyphKey {
                    font_id,
                    font_index,
                    glyph_id: g.glyph_id,
                    variation_coords: g.variation_coords.clone(),
                };
                if let Some(data) =
                    preprocess_glyph(bytes, font_index, g.glyph_id, &g.variation_coords)
                {
                    cache.insert(key, data);
                }
            }
        });
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    fn build_instances(
        &self,
        glyphs: &[RenderedGlyph],
        params: &DrawParams<'_>,
    ) -> Vec<GlyphInstance> {
        let snap = {
            let t = super::text_tuning();
            t.enabled && t.snap
        };
        let mut instances = Vec::new();
        for g in glyphs {
            let key = GlyphKey {
                font_id: g.font_id,
                font_index: g.font_index,
                glyph_id: g.glyph_id,
                variation_coords: g.variation_coords.clone(),
            };
            let Some(cached) = self.cache.get(&key) else {
                continue;
            };
            let Some(atlas_offset) = self.atlas.get_offset(&key) else {
                continue;
            };
            let bbox = cached.band_data.bbox;
            let upem_f = if cached.band_data.units_per_em == 0 {
                1000.0_f32
            } else {
                f32::from(cached.band_data.units_per_em)
            };
            let scale = g.size / upem_f;
            let base_y = params.origin.1 + g.y;
            let pen_y = if snap {
                snap_to_physical_grid(base_y, params.scale)
            } else {
                base_y
            };
            let ink_left = bbox.x_min.mul_add(scale, params.origin.0 + g.x);
            let origin_x = if snap {
                snap_to_physical_grid(ink_left, params.scale)
            } else {
                ink_left
            };
            let color = glyph_color(g, params);
            instances.push(GlyphInstance {
                origin: [origin_x, bbox.y_max.mul_add(-scale, pen_y)],
                size: [bbox.width() * scale, bbox.height() * scale],
                color,
                atlas_offset,
                polarity: polarity_flag(color, self.format),
            });
        }
        instances
    }
}

fn snap_to_physical_grid(logical: f32, scale: f32) -> f32 {
    if scale <= 0.0 {
        return logical;
    }
    (logical * scale).round() / scale
}

fn glyph_color(glyph: &RenderedGlyph, params: &DrawParams<'_>) -> [f32; 4] {
    let color = params.color.unwrap_or(glyph.color);
    [color.r, color.g, color.b, color.a]
}

fn make_bind_group(
    encoder: &mut SplitEncoder,
    params: &RunFrame<'_>,
    bgl: &wgpu::BindGroupLayout,
    atlas_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let tuning = super::text_tuning();
    let surface_uniform = SurfaceUniform {
        surface_size: [
            params.surface_width.max(1.0),
            params.surface_height.max(1.0),
        ],
        scale: params.scale.max(1.0),
        dilation: tuning.dilation,
        sharpen: tuning.sharpen,
        gamma_lod: tuning.gamma_light_on_dark,
        gamma_dol: tuning.gamma_dark_on_light,
        enabled: f32::from(u8::from(tuning.enabled)),
    };
    let (ubuf, offset, _) = encoder.uniform(&surface_uniform);
    params.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("slug-bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &ubuf,
                    offset: u64::from(offset),
                    size: wgpu::BufferSize::new(size_of::<SurfaceUniform>() as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: atlas_buf.as_entire_binding(),
            },
        ],
    })
}
