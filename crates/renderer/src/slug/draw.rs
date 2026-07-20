//! [`SlugTextPipeline`] — GPU draw calls for Slug text rendering.
//!
//! Phase 2 (M2+M3): one atlas buffer per frame; one instanced draw call per
//! paragraph instead of N per-glyph draws.

use std::collections::HashMap;

use sigil_hal::{Color, Rect, RenderedGlyph};
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
    /// Device pixel ratio (physical px per logical px). The shader scales its glyph
    /// coverage-AA ramp by this so edges are one physical pixel wide — sharp on high-DPI.
    /// 1.0 for offscreen/record targets that render at viewport resolution.
    pub scale: f32,
    /// Paragraph to render.
    pub paragraph: &'a Paragraph,
    /// Pre-positioned glyphs for this paragraph, when carried by a scene command.
    pub glyphs: Option<&'a [RenderedGlyph]>,
    /// Top-left origin of the paragraph in viewport coordinates.
    pub origin: (f32, f32),
    /// Optional color override applied to every glyph.
    pub color: Option<Color>,
    /// Optional clip rectangle in viewport (logical) coordinates. When set, the
    /// glyph pass is scissored to it so a paragraph taller/wider than its viewport
    /// (a scrolled editor body, an `overflow: hidden` pane) is cut at the box edge
    /// per-pixel instead of painting over siblings. `None` = no scissor.
    pub clip: Option<Rect>,
}

/// CPU-side uniform mirroring the WGSL `SurfaceParams` struct (32 bytes).
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct SurfaceUniform {
    surface_size: [f32; 2],
    scale: f32,
    dilation: f32,
    sharpen: f32,
    /// Stem-darkening gamma for light-on-dark glyphs (selected per-instance).
    gamma_lod: f32,
    /// Stem-darkening gamma for dark-on-light glyphs (selected per-instance).
    gamma_dol: f32,
    enabled: f32,
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
    /// Contrast polarity of this glyph over its backdrop: `1.0` = light-on-dark,
    /// `0.0` = dark-on-light. Selects which per-polarity stem-darkening gamma the
    /// fragment shader applies. Occupies the trailing pad slot, so the 40-byte
    /// instance stride is unchanged.
    polarity: f32,
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
        let mut encoder = params
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("slug-encoder"),
            });
        self.record_paragraph(&mut encoder, params)?;
        params.queue.submit(core::iter::once(encoder.finish()));
        Ok(())
    }

    /// Record the paragraph's draw into a caller-owned `encoder` instead of
    /// self-submitting. The caller submits the encoder — letting an embedding
    /// host (e.g. a game engine) order text *after* its own passes in one submission.
    /// Composites over existing content (`LoadOp::Load`).
    ///
    /// # Errors
    ///
    /// Currently infallible; `Err` is reserved for future GPU failures.
    ///
    /// # Panics
    ///
    /// Panics if the paragraph contains more than `u32::MAX` glyphs.
    pub fn record_paragraph(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        params: &DrawParams<'_>,
    ) -> Result<(), RendererError> {
        let owned_glyphs;
        let glyphs = if let Some(glyphs) = params.glyphs {
            glyphs
        } else {
            owned_glyphs = params.paragraph.rendered_glyphs();
            &owned_glyphs
        };
        if glyphs.is_empty() {
            return Ok(());
        }
        self.ensure_preprocessed(glyphs, params.paragraph);
        self.atlas.rebuild(params.device, params.queue, &self.cache);
        let instances = self.build_instances(glyphs, params);
        if instances.is_empty() {
            return Ok(());
        }
        let count = u32::try_from(instances.len()).expect("paragraph glyph count overflows u32");
        let instance_buf = upload_instances(params.device, &instances);
        let bind_group = make_bind_group(params, &self.bgl, self.atlas.buffer());
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
        // Scissor the glyph draw to the active clip so a paragraph larger than its
        // box (a scrolled editor body, an overflow-hidden pane) is cut per-pixel at
        // the edge instead of painting over siblings. Empty clip ⇒ nothing to paint.
        if !apply_clip_scissor(&mut pass, params) {
            return Ok(());
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.draw(0..6, 0..count);
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
        // Pixel snapping is part of the tuned ("cleartext") path; the A/B baseline
        // and an explicit snap-off both render at the true fractional positions.
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
            // Poor-man's hinting for an unhinted, resolution-independent cache:
            // snap to the physical pixel grid so glyph features land on whole
            // pixels instead of smearing across two at a random sub-pixel phase.
            //   • Y: snap the BASELINE (pen_y) only, then offset the glyph top by
            //     the fractional ascent — every glyph on the line shares one
            //     pixel-aligned row (snapping each glyph top independently would
            //     jitter the baseline).
            //   • X: snap each glyph's LEFT INK EDGE (origin after the bearing) to
            //     the grid, so the leading vertical stem — the dominant feature of
            //     most letters — sits on a pixel boundary and renders crisp. The
            //     cost is up to 1px of advance unevenness on proportional text,
            //     which is the standard small-text trade and reads better than the
            //     gray smear it replaces.
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

/// Upload a slice of [`GlyphInstance`] into a fresh `VERTEX` buffer.
fn upload_instances(device: &wgpu::Device, instances: &[GlyphInstance]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("slug-instances"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

/// Round a logical-space coordinate so it lands on a physical pixel boundary.
///
/// `scale` is physical px per logical px (the surface scale). Snapping the pen
/// position removes the random sub-pixel phase that makes unhinted glyph stems
/// render unevenly across a line. A no-op at `scale == 1.0` snaps to logical
/// integer pixels; `scale <= 0.0` is treated as "don't snap" (defensive).
fn snap_to_physical_grid(logical: f32, scale: f32) -> f32 {
    if scale <= 0.0 {
        return logical;
    }
    (logical * scale).round() / scale
}

/// Apply the active clip (if any) as a GPU scissor on `pass`. Returns `false`
/// when the clip is non-empty but resolves to an empty region — the caller skips
/// the draw entirely. No clip ⇒ `true` (paint unclipped).
fn apply_clip_scissor(pass: &mut wgpu::RenderPass<'_>, params: &DrawParams<'_>) -> bool {
    let Some(clip) = params.clip else {
        return true;
    };
    match scissor_rect(
        clip,
        params.scale,
        params.surface_width,
        params.surface_height,
    ) {
        Some((x, y, w, h)) => {
            pass.set_scissor_rect(x, y, w, h);
            true
        }
        None => false,
    }
}

/// Convert a logical-pixel clip rect into a physical-pixel scissor `(x, y, w, h)`,
/// clamped to the framebuffer. `None` when the resulting region is empty (the clip
/// fell entirely outside the target). Casts are guarded by the clamps above them.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn scissor_rect(
    clip: Rect,
    scale: f32,
    surface_w: f32,
    surface_h: f32,
) -> Option<(u32, u32, u32, u32)> {
    let fb_w = (surface_w * scale).max(0.0);
    let fb_h = (surface_h * scale).max(0.0);
    let x0 = (clip.origin.x * scale).floor().clamp(0.0, fb_w);
    let y0 = (clip.origin.y * scale).floor().clamp(0.0, fb_h);
    let x1 = ((clip.origin.x + clip.width) * scale)
        .ceil()
        .clamp(0.0, fb_w);
    let y1 = ((clip.origin.y + clip.height) * scale)
        .ceil()
        .clamp(0.0, fb_h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

fn glyph_color(glyph: &RenderedGlyph, params: &DrawParams<'_>) -> [f32; 4] {
    let color = params.color.unwrap_or(glyph.color);
    [color.r, color.g, color.b, color.a]
}

/// Classify a glyph's contrast polarity from its own colour: `1.0` for
/// light-on-dark (bright text, assumed dark backdrop), `0.0` for dark-on-light.
///
/// The instance colour is already in the render target's blend space, so the
/// Rec.709 luminance is decoded back to a *perceptual* value before thresholding
/// at mid-grey: on an sRGB target the channels are linear-light (encode with the
/// sRGB transfer curve first); on a `Unorm` target they are already sRGB-encoded
/// (use directly). UI text sits near the black/white extremes where the choice is
/// unambiguous; the space-correct decode keeps mid-grey text classified sanely.
fn polarity_flag(color: [f32; 4], format: wgpu::TextureFormat) -> f32 {
    let lum = 0.0722f32.mul_add(color[2], 0.7152f32.mul_add(color[1], 0.2126 * color[0]));
    let perceptual = if format.is_srgb() {
        linear_to_srgb(lum)
    } else {
        lum
    };
    f32::from(u8::from(perceptual >= 0.5))
}

/// Encode a linear-light channel to sRGB — the inverse of the renderer's
/// `srgb_channel_to_linear` decode, used to recover a perceptual luminance for
/// polarity classification.
fn linear_to_srgb(value: f32) -> f32 {
    let v = value.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055f32.mul_add(v.powf(1.0 / 2.4), -0.055)
    }
}

/// Build the per-frame bind group: surface uniform (binding 0) + atlas (binding 1).
fn make_bind_group(
    params: &DrawParams<'_>,
    bgl: &wgpu::BindGroupLayout,
    atlas_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    // Snapshot the live text tuning so a debug panel can A/B these against the
    // running window. `enabled` lowers to 0/1 for the shader's A/B gate.
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
