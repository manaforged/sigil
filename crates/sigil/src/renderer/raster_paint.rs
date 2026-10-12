use crate::geometry::Rect;
use crate::hal::RenderedGlyph;
use crate::style::Color;
use crate::text::Paragraph;

use crate::renderer::fill_paint::setup::begin_pass;
use crate::renderer::raster::{Hinters, RasterKey, subpixel_quantize};
pub(crate) use crate::renderer::slug::RunFrame as RasterFrame;
use crate::renderer::split_encoder::SplitEncoder;

mod atlas;
mod setup;

use atlas::RasterAtlas;
#[cfg(any(test, feature = "kittest"))]
pub use atlas::set_test_atlas_size_override;
#[cfg(any(test, feature = "kittest"))]
pub(crate) use atlas::test_atlas_size_override;
use setup::{SurfaceParams, UNIT_QUAD_VERTICES, bind_group, bind_group_layout, pipeline};
use wgpu::util::DeviceExt as _;

pub(crate) struct RasterDrawParams<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) scale: f32,
    pub(crate) paragraph: &'a Paragraph,
    pub(crate) glyphs: &'a [RenderedGlyph],
    pub(crate) origin: (f32, f32),
    pub(crate) color: Color,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub(crate) struct RasterInstance {
    quad_rect: [f32; 4],
    uv_rect: [f32; 4],
    color: [f32; 4],
}

const _: () = assert!(core::mem::size_of::<RasterInstance>() == 48);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RasterGroup {
    pub(crate) clip: Option<Rect>,
    pub(crate) range: core::ops::Range<u32>,
}

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

enum GlyphOutcome {
    Placed(RasterInstance),
    Blank,
    AtlasFull,
}

impl RasterTextPipeline {
    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let bgl = bind_group_layout(device);
        Self {
            format: surface_format,
            pipeline: pipeline(device, &bgl, surface_format),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("sigil-renderer raster text sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            quad_vb: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sigil-renderer raster text unit quad"),
                contents: bytemuck::cast_slice(&UNIT_QUAD_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }),
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

    pub(crate) fn begin_frame(&mut self, queue: &wgpu::Queue) {
        self.atlas.begin_frame(queue);
    }

    pub(crate) fn place_paragraph(
        &mut self,
        params: &RasterDrawParams<'_>,
    ) -> (Vec<RasterInstance>, Vec<RenderedGlyph>) {
        if params.glyphs.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let mut instances = Vec::new();
        let mut leftover = Vec::new();
        let mut claimed = vec![false; params.glyphs.len()];
        let mut any_face = false;
        params
            .paragraph
            .for_each_font(|font_id, font_bytes, font_index| {
                any_face = true;
                for (g, slot) in params.glyphs.iter().zip(&mut claimed) {
                    if g.font_id != font_id || g.font_index != font_index {
                        continue;
                    }
                    *slot = true;
                    match self.place_one(params, font_bytes, g) {
                        GlyphOutcome::Placed(instance) => instances.push(instance),
                        GlyphOutcome::Blank => {}
                        GlyphOutcome::AtlasFull => leftover.push(g.clone()),
                    }
                }
            });
        if !any_face {
            return (Vec::new(), params.glyphs.to_vec());
        }
        leftover.extend(
            params
                .glyphs
                .iter()
                .zip(&claimed)
                .filter(|(_, claimed)| !**claimed)
                .map(|(g, _)| g.clone()),
        );
        (instances, leftover)
    }

    #[allow(clippy::cast_precision_loss)]
    fn place_one(
        &mut self,
        params: &RasterDrawParams<'_>,
        font_bytes: &[u8],
        g: &RenderedGlyph,
    ) -> GlyphOutcome {
        let scale = params.scale.max(1.0);
        let physical_pen_x = (params.origin.0 + g.x) * scale;
        let (bin, carry) = subpixel_quantize(physical_pen_x);
        let key = RasterKey::new(g, g.size * scale, bin);
        let placement = if let Some(p) = self.atlas.get(&key) {
            p
        } else {
            if self.atlas.blank.contains(&key) {
                return GlyphOutcome::Blank;
            }
            let Some(mask) = self
                .hinters
                .rasterize(font_bytes, &key, &g.variation_coords, g.skew)
            else {
                self.atlas.blank.insert(key);
                return GlyphOutcome::Blank;
            };
            let Some(p) = self.atlas.insert(params.device, params.queue, key, mask) else {
                return GlyphOutcome::AtlasFull;
            };
            p
        };
        let quad_x = (physical_pen_x + carry).floor() + placement.left as f32;
        let quad_y = ((params.origin.1 + g.y) * scale).round() - placement.top as f32;
        let c = params.color;
        GlyphOutcome::Placed(RasterInstance {
            quad_rect: [
                quad_x / scale,
                quad_y / scale,
                placement.width as f32 / scale,
                placement.height as f32 / scale,
            ],
            uv_rect: placement.px,
            color: [c.r, c.g, c.b, c.a],
        })
    }

    pub(crate) fn draw_batch(
        &mut self,
        encoder: &mut SplitEncoder,
        frame: &RasterFrame<'_>,
        instances: &mut [RasterInstance],
        groups: &[RasterGroup],
    ) {
        if instances.is_empty() {
            return;
        }
        normalize_uvs(instances, self.atlas.size);
        let (uniform, offset, _) = encoder.uniform(&SurfaceParams {
            surface_size: [frame.surface_width.max(1.0), frame.surface_height.max(1.0)],
            _pad: [0.0, 0.0],
        });
        let view = &self.atlas.view;
        if self
            .uniforms
            .as_ref()
            .is_none_or(|(buffer, atlas, _)| *buffer != uniform || atlas != view)
        {
            let group = bind_group(frame.device, &self.bgl, &uniform, view, &self.sampler);
            self.uniforms = Some((uniform, view.clone(), group));
        }
        let Some((_, _, bind)) = self.uniforms.as_ref() else {
            return;
        };
        let uploaded = encoder.upload(instances);
        let size = frame.view.texture().size();
        let pass = begin_pass(encoder, "raster-text-pass", frame.view, wgpu::LoadOp::Load);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[offset]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, uploaded.buffer.slice(uploaded.bytes));
        let viewport = (frame.surface_width, frame.surface_height);
        for group in groups {
            if group.clip.is_none() {
                pass.set_scissor_rect(0, 0, size.width, size.height);
            }
            if crate::renderer::scissor::apply(pass, group.clip, frame.scale, viewport) {
                pass.draw(0..6, group.range.clone());
            }
        }
        pass.set_scissor_rect(0, 0, size.width, size.height);
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

#[cfg(test)]
#[path = "raster_paint/tests.rs"]
mod tests;
