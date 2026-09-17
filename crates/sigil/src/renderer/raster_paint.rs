use crate::hal::{Color, Rect, RenderedGlyph};
use crate::text::Paragraph;

use crate::renderer::raster::{self, Hinters, RasterKey};
use crate::renderer::split_encoder::SplitEncoder;

mod atlas;
mod setup;

#[doc(hidden)]
pub use atlas::set_test_atlas_size_override;
pub(crate) use atlas::test_atlas_size_override;
use atlas::{Placement, RasterAtlas};
use setup::{
    SurfaceParams, UNIT_QUAD_VERTEX_COUNT, bind_group, bind_group_layout, pipeline, sampler,
    unit_quad,
};

pub(crate) struct RasterDrawParams<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) surface_width: f32,
    pub(crate) surface_height: f32,
    pub(crate) scale: f32,
    pub(crate) paragraph: &'a Paragraph,
    pub(crate) glyphs: &'a [RenderedGlyph],
    pub(crate) origin: (f32, f32),
    pub(crate) color: Color,
    pub(crate) clip: Option<Rect>,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub(crate) struct RasterInstance {
    quad_rect: [f32; 4],
    uv_rect: [f32; 4],
    color: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<RasterInstance>() == 48);

pub(crate) struct RasterTextPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad_vb: wgpu::Buffer,
    atlas: RasterAtlas,
    hinters: Hinters,
    uniforms: Option<(wgpu::Buffer, wgpu::TextureView, wgpu::BindGroup)>,
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
            hinters: Hinters::default(),
            uniforms: None,
            bgl,
        }
    }

    #[must_use]
    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn record_paragraph(
        &mut self,
        encoder: &mut SplitEncoder,
        params: &RasterDrawParams<'_>,
    ) -> Vec<RenderedGlyph> {
        if params.glyphs.is_empty() {
            return Vec::new();
        }
        let mut all_instances: Vec<RasterInstance> = Vec::new();
        let mut leftover: Vec<RenderedGlyph> = Vec::new();
        let mut claimed = vec![false; params.glyphs.len()];
        let mut any_face = false;
        params
            .paragraph
            .for_each_font(|font_id, font_bytes, font_index| {
                any_face = true;
                let result = self.place_glyphs_for_face(params, font_id, font_bytes, font_index);
                all_instances.extend(result.instances);
                leftover.extend(result.leftover);
                for idx in result.claimed_indices {
                    if let Some(slot) = claimed.get_mut(idx) {
                        *slot = true;
                    }
                }
            });
        if !any_face {
            return params.glyphs.to_vec();
        }
        for (g, was_claimed) in params.glyphs.iter().zip(&claimed) {
            if !was_claimed {
                leftover.push(g.clone());
            }
        }
        normalize_uvs(&mut all_instances, self.atlas.size());
        if !all_instances.is_empty() {
            self.draw_instances(encoder, params, &all_instances);
        }
        leftover
    }

    fn place_glyphs_for_face(
        &mut self,
        params: &RasterDrawParams<'_>,
        font_id: u64,
        font_bytes: &[u8],
        font_index: u32,
    ) -> FaceRasterResult {
        let color = [
            params.color.r,
            params.color.g,
            params.color.b,
            params.color.a,
        ];
        let mut instances = Vec::new();
        let mut leftover = Vec::new();
        let mut claimed_indices = Vec::new();
        for (idx, g) in params.glyphs.iter().enumerate() {
            if g.font_id != font_id || g.font_index != font_index {
                continue;
            }
            claimed_indices.push(idx);
            match self.place_one(params, font_bytes, g) {
                GlyphOutcome::Placed(instance) => {
                    instances.push(RasterInstance { color, ..instance });
                }
                GlyphOutcome::Blank => {}
                GlyphOutcome::AtlasFull => leftover.push(g.clone()),
            }
        }
        FaceRasterResult {
            instances,
            leftover,
            claimed_indices,
        }
    }

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
            let Some(mask) = self.hinters.rasterize(font_bytes, &key) else {
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
            physical_pen_x + carry,
            physical_baseline_y,
            scale,
        ))
    }

    fn draw_instances(
        &mut self,
        encoder: &mut SplitEncoder,
        params: &RasterDrawParams<'_>,
        instances: &[RasterInstance],
    ) {
        let (uniform, offset, _) = encoder.uniform(&SurfaceParams {
            surface_size: [
                params.surface_width.max(1.0),
                params.surface_height.max(1.0),
            ],
            _pad: [0.0, 0.0],
        });
        self.prepare_uniforms(params.device, uniform);
        let (_, _, bg) = self.uniforms.as_ref().expect("raster uniforms initialized");
        let instances = encoder.upload(instances);
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        if !apply_clip_scissor(pass, params) {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[offset]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(instances.bytes));
        pass.draw(0..UNIT_QUAD_VERTEX_COUNT, 0..instances.count);
    }

    fn prepare_uniforms(&mut self, device: &wgpu::Device, uniform: wgpu::Buffer) {
        let view = self.atlas.view();
        if self
            .uniforms
            .as_ref()
            .is_none_or(|(buffer, atlas, _)| *buffer != uniform || atlas != view)
        {
            let group = bind_group(device, &self.bgl, &uniform, view, &self.sampler);
            self.uniforms = Some((uniform, view.clone(), group));
        }
    }
}

struct FaceRasterResult {
    instances: Vec<RasterInstance>,
    leftover: Vec<RenderedGlyph>,
    claimed_indices: Vec<usize>,
}

enum GlyphOutcome {
    Placed(RasterInstance),
    Blank,
    AtlasFull,
}

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
        uv_rect: placement.px,
        color: [0.0; 4],
    }
}

fn normalize_uvs(instances: &mut [RasterInstance], atlas_size: u32) {
    #[allow(clippy::cast_precision_loss)]
    let s = atlas_size as f32;
    for i in instances {
        for c in &mut i.uv_rect {
            *c /= s;
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn subpixel_quantize(physical_pen_x: f32) -> (u8, f32) {
    let bins = f32::from(raster::SUBPIXEL_BINS);
    let q = (physical_pen_x.rem_euclid(1.0) * bins).round();
    if q >= bins { (0, 1.0) } else { (q as u8, 0.0) }
}

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
#[path = "raster_paint/tests.rs"]
mod tests;
