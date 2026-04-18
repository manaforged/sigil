//! [`SlugTextPipeline`] — GPU draw calls for Slug text rendering.
//!
//! Phase 2 (M2+M3): one atlas buffer per frame; one instanced draw call per
//! paragraph instead of N per-glyph draws.

use std::collections::HashMap;

use sigil_hal::{Color, RenderedGlyph};
use sigil_text::Paragraph;
use wgpu::util::DeviceExt as _;

use crate::error::RendererError;
use crate::slug::{
    atlas::GlyphAtlas,
    bands::preprocess_glyph,
    cache::{GlyphCache, GlyphKey},
    pipeline::{create_bind_group_layout, create_pipeline},
};

/// Parameters for one [`SlugTextPipeline::draw_paragraph`] call.
pub struct DrawParams<'a> {
    /// wgpu device (for buffer/bind-group creation).
    pub device: &'a wgpu::Device,
    /// wgpu queue (for command submission).
    pub queue: &'a wgpu::Queue,
    /// Render target — caller acquires the surface texture and creates this view.
    pub view: &'a wgpu::TextureView,
    /// Viewport width in the same coordinate space as glyph positions.
    pub surface_width: f32,
    /// Viewport height in the same coordinate space as glyph positions.
    pub surface_height: f32,
    /// Paragraph to render.
    pub paragraph: &'a Paragraph,
    /// Top-left origin of the paragraph in viewport coordinates.
    pub origin: (f32, f32),
    /// Optional color override applied to every glyph.
    pub color: Option<Color>,
}

/// CPU-side uniform mirroring the WGSL `SurfaceParams` struct (16 bytes).
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct SurfaceUniform {
    surface_size: [f32; 2],
    _pad: [f32; 2],
}

/// Per-instance glyph data for the GPU instance vertex buffer (40 bytes).
///
/// Each field maps to a vertex shader `@location` attribute via step mode
/// `Instance`. The `atlas_offset` is a float-index into the shared atlas
/// storage buffer (i.e. byte offset ÷ 4).
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct GlyphInstance {
    origin: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    atlas_offset: u32,
    _pad: u32,
}

/// Six UV vertices for two triangles covering \[0,1\]².
const QUAD_UVS: &[f32] = &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];

/// GPU-side Slug text pipeline.
///
/// Owns the wgpu render pipeline, bind group layout, glyph band-data cache,
/// a reusable quad vertex buffer, and the per-frame glyph atlas.
/// Construct once per surface format; reuse across frames.
pub struct SlugTextPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    cache: GlyphCache,
    quad_vb: wgpu::Buffer,
    atlas: GlyphAtlas,
}

impl SlugTextPipeline {
    /// Create a new pipeline for `surface_format`.
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

    /// Surface format this pipeline was created for.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Draw all glyphs in `paragraph` onto `params.view`.
    ///
    /// Composites over existing content (`LoadOp::Load`). The caller must
    /// acquire the surface texture, create the view, and present the frame.
    ///
    /// # Errors
    ///
    /// Currently infallible; `Err` is reserved for future GPU submission failures.
    ///
    /// # Panics
    ///
    /// Panics if the paragraph contains more than `u32::MAX` glyphs, which
    /// cannot occur in practice.
    pub fn draw_paragraph(&mut self, params: &DrawParams<'_>) -> Result<(), RendererError> {
        let glyphs = params.paragraph.rendered_glyphs();
        if glyphs.is_empty() {
            return Ok(());
        }

        self.ensure_preprocessed(&glyphs, params.paragraph);
        self.atlas.rebuild(params.device, params.queue, &self.cache);

        let instances = self.build_instances(&glyphs, params);
        if instances.is_empty() {
            return Ok(());
        }

        let count = u32::try_from(instances.len()).expect("paragraph glyph count overflows u32");
        let instance_buf = upload_instances(params.device, &instances);
        let bind_group = make_bind_group(params, &self.bgl, self.atlas.buffer());
        self.submit_draw(params, &instance_buf, &bind_group, count);
        Ok(())
    }

    /// Preprocess any glyphs not yet in the cache.
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

    /// Build per-instance CPU data for every cached glyph in `glyphs`.
    #[allow(clippy::cast_precision_loss)]
    fn build_instances(
        &self,
        glyphs: &[RenderedGlyph],
        params: &DrawParams<'_>,
    ) -> Vec<GlyphInstance> {
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
            instances.push(GlyphInstance {
                origin: [
                    bbox.x_min.mul_add(scale, params.origin.0 + g.x),
                    bbox.y_max.mul_add(-scale, params.origin.1 + g.y),
                ],
                size: [bbox.width() * scale, bbox.height() * scale],
                color: glyph_color(g, params),
                atlas_offset,
                _pad: 0,
            });
        }
        instances
    }

    /// Encode and submit one instanced draw call for the paragraph.
    fn submit_draw(
        &self,
        params: &DrawParams<'_>,
        instance_buf: &wgpu::Buffer,
        bind_group: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        let mut encoder = params
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("slug-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("slug-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: params.view,
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
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_vb.slice(..));
            pass.set_vertex_buffer(1, instance_buf.slice(..));
            pass.draw(0..6, 0..instance_count);
        }
        params.queue.submit(core::iter::once(encoder.finish()));
    }
}

/// Upload a slice of [`GlyphInstance`] into a fresh `VERTEX` buffer.
fn upload_instances(device: &wgpu::Device, instances: &[GlyphInstance]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("slug-instances"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

fn glyph_color(glyph: &RenderedGlyph, params: &DrawParams<'_>) -> [f32; 4] {
    let color = params.color.unwrap_or(glyph.color);
    [color.r, color.g, color.b, color.a]
}

/// Build the per-frame bind group: surface uniform (binding 0) + atlas (binding 1).
fn make_bind_group(
    params: &DrawParams<'_>,
    bgl: &wgpu::BindGroupLayout,
    atlas_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let surface_uniform = SurfaceUniform {
        surface_size: [
            params.surface_width.max(1.0),
            params.surface_height.max(1.0),
        ],
        _pad: [0.0; 2],
    };
    let ubuf = params
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slug-surface-uniform"),
            contents: bytemuck::bytes_of(&surface_uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
    params.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("slug-bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: atlas_buf.as_entire_binding(),
            },
        ],
    })
}
