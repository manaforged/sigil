#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
use crate::renderer::glyph_place::{pen_shift, text_gamma};
use crate::renderer::scene_instance::BlendSpace;
use std::collections::HashMap;

use crate::geometry::{Point, Rect};
use crate::hal::RenderedGlyph;
use crate::scene::command::effects::transform_point;
use crate::style::Color;
use crate::text::Paragraph;
use wgpu::util::DeviceExt as _;

#[cfg(any(test, feature = "kittest"))]
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

#[cfg_attr(not(any(test, feature = "kittest")), allow(dead_code))]
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
    pub transform: [[f32; 3]; 2],
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
    enabled: f32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub(crate) struct GlyphInstance {
    origin: [f32; 2],
    axis_x: [f32; 2],
    axis_y: [f32; 2],
    color: [f32; 4],
    atlas_offset: u32,
    gamma: f32,
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

    #[cfg(any(test, feature = "kittest"))]
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

    #[cfg(any(test, feature = "kittest"))]
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
            let key = GlyphKey::of(g);
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
                let key = GlyphKey::of(g);
                if let Some(data) =
                    preprocess_glyph(bytes, font_index, g.glyph_id, &g.variation_coords, g.skew)
                {
                    cache.insert(key, data);
                }
            }
        });
    }

    fn build_instances(
        &self,
        glyphs: &[RenderedGlyph],
        params: &DrawParams<'_>,
    ) -> Vec<GlyphInstance> {
        glyphs
            .iter()
            .filter_map(|g| self.instance(g, params))
            .collect()
    }

    fn instance(&self, glyph: &RenderedGlyph, params: &DrawParams<'_>) -> Option<GlyphInstance> {
        let key = GlyphKey::of(glyph);
        let cached = self.cache.get(&key)?;
        let atlas_offset = self.atlas.get_offset(&key)?;
        let bbox = cached.band_data.bbox;
        let upem = match cached.band_data.units_per_em {
            0 => 1000.0_f32,
            units => f32::from(units),
        };
        let scale = glyph.size / upem;
        let pen = (params.origin.0 + glyph.x, params.origin.1 + glyph.y);
        let shift = pen_shift(params.transform, Point::new(pen.0, pen.1), params.scale);
        let pen = (pen.0 + shift.x, pen.1 + shift.y);
        let corner = (
            bbox.x_min.mul_add(scale, pen.0),
            bbox.y_max.mul_add(-scale, pen.1),
        );
        let [[xx, xy, _], [yx, yy, _]] = params.transform;
        let mapped = transform_point(params.transform, Point::new(corner.0, corner.1));
        let origin = [mapped.x, mapped.y];
        let (width, height) = (bbox.width() * scale, bbox.height() * scale);
        let color = glyph_color(glyph, params);
        Some(GlyphInstance {
            origin,
            axis_x: [xx * width, yx * width],
            axis_y: [xy * height, yy * height],
            color,
            atlas_offset,
            gamma: text_gamma(
                color,
                BlendSpace::from_format(self.format) == BlendSpace::Linear,
            ),
        })
    }
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
