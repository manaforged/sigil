//! Raster glyph-atlas text pipeline — Wave-3 D2 (DR-055).
//!
//! Draws glyphs pre-rasterized by the hinted CPU path (`crate::raster`)
//! through a shared R8 atlas, as a separate pipeline beside
//! [`crate::slug::SlugTextPipeline`] (DR-055 §1: distinct instance layouts,
//! no merged discriminant branch). Selection between this path and vector
//! Slug happens at the draw entry point (`scene_command::draw_handlers`),
//! not in here — this pipeline only knows how to place glyphs it is given.

use hal::{Color, Rect, RenderedGlyph};
use text::Paragraph;

use crate::raster::{self, RasterKey};

mod atlas;
mod setup;

use atlas::{Placement, RasterAtlas};
use setup::{
    UNIT_QUAD_VERTEX_COUNT, bind_group, bind_group_layout, instance_buffer, pipeline, sampler,
    uniform_buffer, unit_quad,
};

/// Parameters for one [`RasterTextPipeline::record_paragraph`] call. Mirrors
/// [`crate::slug::DrawParams`] so the two pipelines stay call-compatible at
/// the draw entry point.
pub(crate) struct RasterDrawParams<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) surface_width: f32,
    pub(crate) surface_height: f32,
    /// Physical pixels per logical pixel — the size masks are rasterized at.
    pub(crate) scale: f32,
    pub(crate) paragraph: &'a Paragraph,
    pub(crate) glyphs: &'a [RenderedGlyph],
    pub(crate) origin: (f32, f32),
    pub(crate) color: Color,
    pub(crate) clip: Option<Rect>,
}

/// Per-instance GPU record for one placed glyph quad (48 bytes).
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct RasterInstance {
    /// `[x, y, w, h]` destination quad, logical pixels.
    quad_rect: [f32; 4],
    /// `[u0, v0, u1, v1]` atlas UV rect, normalised.
    uv_rect: [f32; 4],
    /// `[r, g, b, a]` linear color; alpha pre-multiplies atlas coverage.
    color: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<RasterInstance>() == 48);

/// GPU-side raster text pipeline: owns the render pipeline, sampler, and the
/// shared [`RasterAtlas`]. Construct once per surface format; reuse across
/// frames — the atlas persists cached glyph masks across draws.
pub(crate) struct RasterTextPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad_vb: wgpu::Buffer,
    atlas: RasterAtlas,
}

impl RasterTextPipeline {
    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let bgl = bind_group_layout(device);
        Self {
            format: surface_format,
            pipeline: pipeline(device, &bgl, surface_format),
            sampler: sampler(device),
            quad_vb: unit_quad(device),
            atlas: RasterAtlas::new(device),
            bgl,
        }
    }

    #[must_use]
    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Place every eligible glyph in `params.glyphs` into the atlas and
    /// record a raster draw for them. Returns the glyphs that could not be
    /// placed (atlas full) — the caller draws those through vector Slug
    /// instead, per DR-055's "at most two draws per run".
    pub(crate) fn record_paragraph(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        params: &RasterDrawParams<'_>,
    ) -> Vec<RenderedGlyph> {
        if params.glyphs.is_empty() {
            return Vec::new();
        }
        // `for_each_font`'s callback signature erases the byte-slice lifetime
        // (HRTB `for<'a> FnMut(u64, &'a [u8], u32)`), so the placement work
        // has to happen INSIDE the closure — `font_bytes` cannot escape it.
        let mut result = None;
        params
            .paragraph
            .for_each_font(|font_id, font_bytes, font_index| {
                if result.is_none() {
                    result = Some(self.place_glyphs(params, font_id, font_bytes, font_index));
                }
            });
        let Some((instances, leftover)) = result else {
            return params.glyphs.to_vec();
        };
        if !instances.is_empty() {
            self.draw_instances(encoder, params, &instances);
        }
        leftover
    }

    /// Rasterize/allocate each glyph, splitting into GPU instances (placed)
    /// and leftover [`RenderedGlyph`]s (atlas full — vector fallback).
    fn place_glyphs(
        &mut self,
        params: &RasterDrawParams<'_>,
        font_id: u64,
        font_bytes: &[u8],
        font_index: u32,
    ) -> (Vec<RasterInstance>, Vec<RenderedGlyph>) {
        let color = [
            params.color.r,
            params.color.g,
            params.color.b,
            params.color.a,
        ];
        let mut instances = Vec::with_capacity(params.glyphs.len());
        let mut leftover = Vec::new();
        for g in params.glyphs {
            if g.font_id != font_id || g.font_index != font_index {
                // Defensive: a paragraph carries one font (see text::Paragraph).
                leftover.push(g.clone());
                continue;
            }
            match self.place_one(params, font_bytes, g) {
                GlyphOutcome::Placed(instance) => {
                    instances.push(RasterInstance { color, ..instance });
                }
                GlyphOutcome::Blank => {}
                GlyphOutcome::AtlasFull => leftover.push(g.clone()),
            }
        }
        // AFTER all placements: a mid-loop atlas grow doubles the texture,
        // so UVs can only be normalised against the final size (the reviewer
        // adversarial finding, 2026-07-12).
        normalize_uvs(&mut instances, self.atlas.size());
        (instances, leftover)
    }

    /// Resolve one glyph's atlas placement (cache hit or fresh rasterize +
    /// insert) into a positioned [`RasterInstance`] (color left zeroed —
    /// the caller fills it in).
    fn place_one(
        &mut self,
        params: &RasterDrawParams<'_>,
        font_bytes: &[u8],
        g: &RenderedGlyph,
    ) -> GlyphOutcome {
        let scale = params.scale.max(1.0);
        let physical_px = g.size * scale;
        let physical_pen_x = (params.origin.0 + g.x) * scale;
        let (bin, carry) = subpixel_quantize(physical_pen_x);
        let key = RasterKey {
            font_id: g.font_id,
            font_index: g.font_index,
            glyph_id: g.glyph_id,
            physical_px_bits: physical_px.to_bits(),
            bin,
        };
        let placement = if let Some(p) = self.atlas.get(&key) {
            p
        } else {
            let Some(mask) =
                raster::rasterize_glyph(font_bytes, g.font_index, g.glyph_id, physical_px, bin)
            else {
                return GlyphOutcome::Blank;
            };
            let Some(p) = self.atlas.insert(params.device, params.queue, key, mask) else {
                return GlyphOutcome::AtlasFull;
            };
            p
        };
        let physical_baseline_y = (params.origin.1 + g.y) * scale;
        GlyphOutcome::Placed(quad_instance(
            &placement,
            // The wrap carry shifts the anchor to the next physical pixel —
            // bin 0 of pixel N+1 is the nearest baked mask for frac >= 0.875.
            physical_pen_x + carry,
            physical_baseline_y,
            scale,
        ))
    }

    fn draw_instances(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        params: &RasterDrawParams<'_>,
        instances: &[RasterInstance],
    ) {
        let uniform = uniform_buffer(
            params.device,
            [
                params.surface_width.max(1.0),
                params.surface_height.max(1.0),
            ],
        );
        let bg = bind_group(
            params.device,
            &self.bgl,
            &uniform,
            self.atlas.view(),
            &self.sampler,
        );
        let instance_buf = instance_buffer(params.device, instances);
        let count = u32::try_from(instances.len()).unwrap_or(u32::MAX);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster-text-pass"),
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
        if !apply_clip_scissor(&mut pass, params) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bg, &[]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..count);
    }
}

/// Result of resolving one glyph against the atlas.
enum GlyphOutcome {
    /// Placed and ready to draw.
    Placed(RasterInstance),
    /// No outline (space, control) — dropped silently by both text paths.
    Blank,
    /// Atlas is full (already grown once) — caller falls back to vector Slug.
    AtlasFull,
}

/// Baseline positioning (DR-055 dispatch brief §4): `GlyphMask.left/top` are
/// offsets from the pen position on the baseline (top = rise above
/// baseline, positive up). The sub-pixel X fraction is already baked into
/// the mask via `bin`, so the quad anchors at `floor(pen_x) + left`; the Y
/// axis has no sub-pixel baking, so it snaps to the nearest physical pixel.
/// Both are converted back to logical px (÷ scale) to match the vertex
/// shader's logical-space NDC conversion.
#[allow(clippy::cast_precision_loss)]
fn quad_instance(
    placement: &Placement,
    physical_pen_x: f32,
    physical_baseline_y: f32,
    scale: f32,
) -> RasterInstance {
    let quad_x = physical_pen_x.floor() + placement.left as f32;
    let quad_y = physical_baseline_y.round() - placement.top as f32;
    RasterInstance {
        quad_rect: [
            quad_x / scale,
            quad_y / scale,
            placement.width as f32 / scale,
            placement.height as f32 / scale,
        ],
        // Atlas TEXELS at this point — normalised against the final atlas
        // size in `normalize_uvs` once the whole draw's placements are done
        // (mid-accumulation growth would invalidate early normalisation).
        uv_rect: placement.px,
        color: [0.0; 4],
    }
}

/// Convert accumulated texel-space `uv_rect`s to normalised UVs against the
/// FINAL atlas size for this draw. Must run after every `place_one` call —
/// growth during accumulation is exactly the case this ordering exists for.
fn normalize_uvs(instances: &mut [RasterInstance], atlas_size: u32) {
    #[allow(clippy::cast_precision_loss)]
    let s = atlas_size as f32;
    for i in instances {
        for c in &mut i.uv_rect {
            *c /= s;
        }
    }
}

/// Quantize a physical-pixel pen position into a subpixel mask bin plus the
/// integer carry the anchor must absorb. Nearest-bin rounding wraps at
/// `frac >= 1 - 1/(2*BINS)`: the mask for bin 0 of the NEXT pixel is the
/// closest, so the quad anchor shifts +1px — dropping that carry (plain
/// `% BINS`) misplaces glyphs by up to a full pixel at the wrap boundary.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn subpixel_quantize(physical_pen_x: f32) -> (u8, f32) {
    let bins = f32::from(raster::SUBPIXEL_BINS);
    let q = (physical_pen_x.rem_euclid(1.0) * bins).round();
    if q >= bins { (0, 1.0) } else { (q as u8, 0.0) }
}

/// Apply the active clip (if any) as a GPU scissor. Mirrors
/// `slug::draw::apply_clip_scissor` — small enough that duplicating it here
/// keeps the two pipelines independent per DR-055 §1 (watch the rule of
/// three before extracting a shared helper).
fn apply_clip_scissor(pass: &mut wgpu::RenderPass<'_>, params: &RasterDrawParams<'_>) -> bool {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    /// Wrap boundary: frac >= 0.875 must carry +1px into the anchor, not
    /// silently alias to bin 0 of the CURRENT pixel (review finding 2).
    #[test]
    fn subpixel_wrap_carries_into_anchor() {
        assert_eq!(subpixel_quantize(10.0), (0, 0.0));
        assert_eq!(subpixel_quantize(10.1), (0, 0.0));
        assert_eq!(subpixel_quantize(10.4), (2, 0.0));
        assert_eq!(subpixel_quantize(10.874), (3, 0.0));
        assert_eq!(subpixel_quantize(10.875), (0, 1.0));
        assert_eq!(subpixel_quantize(10.999), (0, 1.0));
    }

    /// Invariant: the quantized placement (anchor + bin offset) stays within
    /// half a bin (0.125px) of the true pen position across the whole range.
    #[test]
    fn quantized_placement_error_bounded_by_half_bin() {
        let bins = f32::from(raster::SUBPIXEL_BINS);
        for i in 0..1000 {
            #[allow(clippy::cast_precision_loss)]
            let pen = 10.0 + (i as f32) / 1000.0;
            let (bin, carry) = subpixel_quantize(pen);
            let placed = (pen + carry).floor() + f32::from(bin) / bins;
            assert!(
                (placed - pen).abs() <= 0.5 / bins + 1e-4,
                "pen {pen}: placed {placed}, err {}",
                (placed - pen).abs()
            );
        }
    }

    /// UVs are normalised against the FINAL atlas size — texel rects divided
    /// by whatever size the atlas ended the accumulation at (review finding 1).
    #[test]
    fn normalize_uvs_divides_by_final_size() {
        let mut instances = vec![
            RasterInstance {
                quad_rect: [0.0; 4],
                uv_rect: [0.0, 0.0, 512.0, 512.0],
                color: [0.0; 4],
            },
            RasterInstance {
                quad_rect: [0.0; 4],
                uv_rect: [1024.0, 0.0, 2048.0, 1024.0],
                color: [0.0; 4],
            },
        ];
        // Simulates growth to 2048 AFTER the first instance was placed.
        normalize_uvs(&mut instances, 2048);
        assert_eq!(instances[0].uv_rect, [0.0, 0.0, 0.25, 0.25]);
        assert_eq!(instances[1].uv_rect, [0.5, 0.0, 1.0, 0.5]);
    }
}
