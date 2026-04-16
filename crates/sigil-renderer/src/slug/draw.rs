//! [`SlugTextPipeline`] — GPU draw calls for Slug text rendering.

use std::collections::HashMap;

use sigil_hal::RenderedGlyph;
use sigil_text::Paragraph;

use crate::error::RendererError;
use crate::slug::{
    bands::{BandData, preprocess_glyph},
    cache::{CachedGlyph, GlyphCache, GlyphKey},
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
    /// Surface width in physical pixels.
    pub surface_width: u32,
    /// Surface height in physical pixels.
    pub surface_height: u32,
    /// Paragraph to render.
    pub paragraph: &'a Paragraph,
    /// Top-left origin of the paragraph in surface pixels.
    pub origin: (f32, f32),
}

/// CPU-side uniform that mirrors the WGSL `GlyphParams` struct (48 bytes).
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct GlyphUniform {
    origin: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    surface_size: [f32; 2],
    _pad: [f32; 2],
}

/// One ready-to-draw glyph: a wgpu bind group referencing per-glyph GPU buffers.
struct GlyphDraw {
    bind_group: wgpu::BindGroup,
}

/// Bundle of references needed to record the render pass.
struct RenderCtx<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    view: &'a wgpu::TextureView,
    pipeline: &'a wgpu::RenderPipeline,
    quad_vb: &'a wgpu::Buffer,
}

/// Six UV vertices for two triangles covering \[0,1\]².
const QUAD_UVS: &[f32] = &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];

/// GPU-side Slug text pipeline.
///
/// Owns the wgpu render pipeline, bind group layout, glyph band-data cache,
/// and a reusable quad vertex buffer.  Construct once per surface format;
/// reuse across frames.
pub struct SlugTextPipeline {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    cache: GlyphCache,
    quad_vb: wgpu::Buffer,
}

impl SlugTextPipeline {
    /// Create a new pipeline for `surface_format`.
    #[must_use]
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        use wgpu::util::DeviceExt as _;
        let bgl = create_bind_group_layout(device);
        let pipeline = create_pipeline(device, &bgl, surface_format);
        let quad_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("slug-quad-vb"),
            contents: bytemuck::cast_slice(QUAD_UVS),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            pipeline,
            bgl,
            cache: GlyphCache::new(),
            quad_vb,
        }
    }

    /// Draw all glyphs in `paragraph` onto `params.view`.
    ///
    /// Composites over existing content (`LoadOp::Load`). The caller must
    /// acquire the surface texture, create the view, and present the frame.
    ///
    /// # Errors
    ///
    /// Currently infallible; `Err` is reserved for future GPU submission failures.
    pub fn draw_paragraph(&mut self, params: &DrawParams<'_>) -> Result<(), RendererError> {
        let glyphs = params.paragraph.rendered_glyphs();
        if glyphs.is_empty() {
            return Ok(());
        }
        self.ensure_preprocessed(&glyphs, params.paragraph);
        let draws = self.build_draws(&glyphs, params);
        if draws.is_empty() {
            return Ok(());
        }
        let ctx = RenderCtx {
            device: params.device,
            queue: params.queue,
            view: params.view,
            pipeline: &self.pipeline,
            quad_vb: &self.quad_vb,
        };
        submit_draws(&ctx, &draws);
        Ok(())
    }

    /// Preprocess any glyphs missing from the cache by calling `for_each_font`.
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

    /// Build per-glyph GPU resources for every cached glyph in `glyphs`.
    #[allow(clippy::cast_precision_loss)]
    fn build_draws(&self, glyphs: &[RenderedGlyph], params: &DrawParams<'_>) -> Vec<GlyphDraw> {
        let sw = params.surface_width as f32;
        let sh = params.surface_height as f32;
        let mut draws = Vec::new();
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
            let uniform = glyph_uniform(g, cached, params.origin, sw, sh);
            draws.push(make_glyph_draw(
                params.device,
                &self.bgl,
                &uniform,
                &cached.band_data,
            ));
        }
        draws
    }
}

/// Compute the per-glyph uniform from layout and cache data.
fn glyph_uniform(
    g: &RenderedGlyph,
    cached: &CachedGlyph,
    origin: (f32, f32),
    sw: f32,
    sh: f32,
) -> GlyphUniform {
    let bbox = cached.band_data.bbox;
    let upem = cached.band_data.units_per_em;
    let upem_f = if upem == 0 {
        1000.0_f32
    } else {
        f32::from(upem)
    };
    let scale = g.size / upem_f;
    // Map design-unit bbox to pixel-space quad (baseline = glyph.y in Parley).
    GlyphUniform {
        origin: [
            bbox.x_min.mul_add(scale, origin.0 + g.x),
            bbox.y_max.mul_add(-scale, origin.1 + g.y),
        ],
        size: [bbox.width() * scale, bbox.height() * scale],
        color: [g.color.r, g.color.g, g.color.b, g.color.a],
        surface_size: [sw, sh],
        _pad: [0.0; 2],
    }
}

/// Create per-glyph GPU buffers and a bind group.
fn make_glyph_draw(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    uniform: &GlyphUniform,
    band_data: &BandData,
) -> GlyphDraw {
    use wgpu::util::DeviceExt as _;
    let ubuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("slug-uniform"),
        contents: bytemuck::bytes_of(uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("slug-bands"),
        contents: band_data.as_bytes(),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("slug-bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: bbuf.as_entire_binding(),
            },
        ],
    });
    GlyphDraw { bind_group: bg }
}

/// Record the render pass and submit it to the queue.
fn submit_draws(ctx: &RenderCtx<'_>, draws: &[GlyphDraw]) {
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("slug-encoder"),
        });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("slug-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ctx.view,
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
        pass.set_pipeline(ctx.pipeline);
        pass.set_vertex_buffer(0, ctx.quad_vb.slice(..));
        for draw in draws {
            pass.set_bind_group(0, &draw.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
    ctx.queue.submit(core::iter::once(encoder.finish()));
}
