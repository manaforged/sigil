use crate::hal::RenderedGlyph;
use crate::profile::count;
use crate::scene::geometry::IDENTITY;
use crate::style::Rgba;
use crate::text::Paragraph;

use crate::renderer::fill_paint::setup::texture_group;
use crate::renderer::raster::{RasterKey, subpixel_quantize};
use crate::renderer::slug::RunFrame;
use crate::renderer::split_encoder::SplitEncoder;

use super::{AtlasPipeline, QuadBatch, QuadGroup};
use super::{QuadInstance, QuadKind};

pub(crate) struct MaskParams<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) scale: f32,
    pub(crate) paragraph: &'a Paragraph,
    pub(crate) glyphs: &'a [RenderedGlyph],
    pub(crate) origin: (f32, f32),
    pub(crate) color: Rgba,
}

enum GlyphOutcome {
    Placed(QuadInstance),
    Blank,
    AtlasFull,
}

pub(super) fn nearest_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("sigil-renderer raster text sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

impl AtlasPipeline {
    pub(crate) fn place_paragraph(
        &mut self,
        params: &MaskParams<'_>,
    ) -> (Vec<QuadInstance>, Vec<RenderedGlyph>) {
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
        params: &MaskParams<'_>,
        font_bytes: &[u8],
        g: &RenderedGlyph,
    ) -> GlyphOutcome {
        let scale = params.scale.max(1.0);
        let physical_pen_x = (params.origin.0 + g.x) * scale;
        let (bin, carry) = subpixel_quantize(physical_pen_x);
        let key = RasterKey::new(g, g.size * scale, bin);
        let placement = if let Some(p) = self.mask_atlas.get(&key) {
            p
        } else {
            if self.mask_atlas.blank.contains(&key) {
                return GlyphOutcome::Blank;
            }
            let Some(mask) = self
                .hinters
                .rasterize(font_bytes, &key, &g.variation_coords, g.skew)
            else {
                self.mask_atlas.blank.insert(key);
                return GlyphOutcome::Blank;
            };
            let Some(p) = self
                .mask_atlas
                .insert(params.device, params.queue, key, mask)
            else {
                return GlyphOutcome::AtlasFull;
            };
            p
        };
        let quad_x = (physical_pen_x + carry).floor() + placement.left as f32;
        let quad_y = ((params.origin.1 + g.y) * scale).round() - placement.top as f32;
        let c = params.color;
        GlyphOutcome::Placed(QuadInstance::new(
            (
                [
                    quad_x / scale,
                    quad_y / scale,
                    placement.width as f32 / scale,
                    placement.height as f32 / scale,
                ],
                placement.px,
            ),
            [c.r, c.g, c.b, c.a],
            (QuadKind::Mask, 1.0, 0.0),
            IDENTITY,
        ))
    }

    pub(crate) fn draw_masks(
        &mut self,
        encoder: &mut SplitEncoder,
        frame: &RunFrame<'_>,
        instances: &mut [QuadInstance],
        groups: &[QuadGroup],
    ) {
        if instances.is_empty() {
            return;
        }
        normalize_uvs(instances, self.mask_atlas.size);
        let view = &self.mask_atlas.view;
        if self
            .mask_group
            .as_ref()
            .is_none_or(|(atlas, _)| atlas != view)
        {
            count(crate::profile::Counter::AllocRasterBindGroups, 1);
            let group = texture_group(
                frame.device,
                "sigil-renderer raster text bind group",
                &self.atlas_layout,
                view,
                &self.mask_sampler,
            );
            self.mask_group = Some((view.clone(), group));
        }
        let Some((_, atlas)) = self.mask_group.clone() else {
            return;
        };
        #[allow(clippy::cast_precision_loss)]
        let size = self.mask_atlas.size as f32;
        self.draw_quads(
            encoder,
            frame,
            &QuadBatch {
                atlas: &atlas,
                atlas_size: (size, size),
                blend: QuadKind::Mask.blend(),
                instances: &*instances,
                groups,
            },
        );
    }
}

fn normalize_uvs(instances: &mut [QuadInstance], atlas_size: u32) {
    #[allow(clippy::cast_precision_loss)]
    let s = atlas_size as f32;
    for i in instances {
        for c in &mut i.uv_rect {
            *c /= s;
        }
    }
}

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;
