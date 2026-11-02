#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use crate::hal::RenderedGlyph;
use crate::icons::{bake, builtin};
use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::quad_paint::QuadKind;
use crate::renderer::raster::{GlyphMask, Hinters, RasterKey, subpixel_quantize};
use crate::style::Rgba8;
use ahash::AHashMap;
use tiny_skia::{FilterQuality, Paint, Pattern, Pixmap, SpreadMode, Transform};

use crate::renderer::quad_paint::BUILTIN_SPRITE_ATLAS_ID;
use crate::renderer::scene_command::{DrawImage, DrawSprite, GlyphRun};
use crate::renderer::sprite_atlas::{
    BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, builtin_atlas_rgba8, builtin_icon_index,
};

use super::bound::CpuBound;
use super::pattern::{PatternPlacement, shader};
use super::shapes::{Brush, decode, encode, stored};
use super::sprite_tint::tint;

const ICON_PX: u32 = 128;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SpriteKey {
    atlas: u32,
    region: [u32; 4],
    size: u32,
    color: Rgba8,
    kind: QuadKind,
}

#[derive(Default)]
pub(super) struct QuadCache {
    masks: AHashMap<RasterKey, (Option<GlyphMask>, u64)>,
    hinters: Hinters,
    images: AHashMap<u64, (Pixmap, u64)>,
    frame: u64,
    atlases: AHashMap<u32, Pixmap>,
    sprites: AHashMap<SpriteKey, Pixmap>,
    pattern: Option<Pixmap>,
}

pub(super) fn from_rgba8(width: u32, height: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    let (dst_px, _) = pixmap.data_mut().as_chunks_mut::<4>();
    let (src_px, _) = rgba.as_chunks::<4>();
    for (dst, src) in dst_px.iter_mut().zip(src_px) {
        let alpha = f32::from(src[3]) / 255.0;
        for (d, s) in dst.iter_mut().zip(src).take(3) {
            *d = stored(decode(*s), alpha);
        }
        dst[3] = src[3];
    }
    Some(pixmap)
}

fn fill_scaled(target: &mut Pixmap, source: &Pixmap, src: Rect, dest: Rect, brush: &Brush<'_>) {
    let Some(dest_rect) = CpuBound::of(target, brush.transform).clip_rect(dest) else {
        return;
    };
    let scale_x = dest.width / src.width.max(f32::EPSILON);
    let scale_y = dest.height / src.height.max(f32::EPSILON);
    let map = Transform::from_row(
        scale_x,
        0.0,
        0.0,
        scale_y,
        src.origin.x.mul_add(-scale_x, dest.origin.x),
        src.origin.y.mul_add(-scale_y, dest.origin.y),
    );
    let paint = Paint {
        shader: Pattern::new(
            source.as_ref(),
            SpreadMode::Pad,
            FilterQuality::Bilinear,
            brush.opacity,
            map,
        ),
        anti_alias: true,
        colorspace: super::shapes::BLEND_SPACE,
        ..Paint::default()
    };
    target.fill_rect(dest_rect, &paint, brush.transform, brush.coverage());
}

pub(super) struct RunPlacement {
    pub(super) origin: Point,
    pub(super) scale: f32,
    pub(super) offset: Point,
    pub(super) clip: Option<Rect>,
    pub(super) table: [[u8; 4]; 256],
}

impl QuadCache {
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        let limits = CacheLimits::DEFAULT;
        self.images
            .retain(|_, (_, used)| *used + limits.image_stale_frames >= frame);
        if self.masks.len() > limits.glyph_masks {
            self.masks
                .retain(|_, (_, used)| *used + limits.glyph_stale_frames >= frame);
        }
    }

    pub(super) fn image(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawImage,
        scale: f32,
        brush: &Brush<'_>,
    ) {
        let Some((source, src_rect, dst_rect)) = draw.placement_at(scale) else {
            return;
        };
        if !self.images.contains_key(&source.id()) {
            let Some(pixmap) = from_rgba8(source.width(), source.height(), source.rgba()) else {
                return;
            };
            self.images.insert(source.id(), (pixmap, self.frame));
        }
        let Some((pixmap, used)) = self.images.get_mut(&source.id()) else {
            return;
        };
        *used = self.frame;
        let pixmap = &*pixmap;
        fill_scaled(target, pixmap, src_rect, dst_rect, brush);
    }

    pub(super) fn sprite(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawSprite,
        dest: Rect,
        brush: &Brush<'_>,
    ) {
        let Some(dims) = self
            .atlas(draw.source.atlas)
            .map(|atlas| (atlas.width() as f32, atlas.height() as f32))
        else {
            return;
        };
        let Some((region, dest)) = draw.source.fitted(dims, dest) else {
            return;
        };
        let key = SpriteKey {
            atlas: draw.source.atlas,
            region: [
                region.origin.x,
                region.origin.y,
                region.width,
                draw.source.phase,
            ]
            .map(f32::to_bits),
            size: region.height.to_bits(),
            color: draw.color,
            kind: QuadKind::from(draw.kind),
        };
        if !self.sprites.contains_key(&key) {
            let Some(atlas) = self.atlas(draw.source.atlas) else {
                return;
            };
            let Some(tinted) = tint(atlas, region, draw) else {
                return;
            };
            self.sprites.insert(key, tinted);
        }
        let Some(sprite) = self.sprites.get(&key) else {
            return;
        };
        let src = Rect::new(
            Point::new(0.0, 0.0),
            sprite.width() as f32,
            sprite.height() as f32,
        );
        fill_scaled(target, sprite, src, dest, brush);
    }

    pub(super) fn draw_run(&mut self, target: &mut Pixmap, run: &GlyphRun, place: &RunPlacement) {
        let Some(paragraph) = run.paragraph() else {
            return;
        };
        let glyphs = run.positioned_glyphs();
        let frame = self.frame;
        if glyphs
            .iter()
            .any(|g| !self.masks.contains_key(&place_glyph(g, place).0))
        {
            paragraph.for_each_font(|font_id, bytes, font_index| {
                for g in glyphs {
                    let (k, _, _) = place_glyph(g, place);
                    if k.font_id == font_id
                        && k.font_index == font_index
                        && !self.masks.contains_key(&k)
                    {
                        let mask = self
                            .hinters
                            .rasterize(bytes, &k, &g.variation_coords, g.skew);
                        self.masks.insert(k, (mask, frame));
                    }
                }
            });
        }
        let bounds = pixel_clip(place.clip, place.scale, place.offset, target);
        for g in glyphs {
            let (k, x, y) = place_glyph(g, place);
            if let Some((mask, used)) = self.masks.get_mut(&k) {
                *used = frame;
                if let Some(mask) = mask {
                    blit(target, mask, (x, y), bounds, &place.table);
                }
            }
        }
    }

    fn atlas(&mut self, id: u32) -> Option<&Pixmap> {
        if !self.atlases.contains_key(&id) {
            let pixmap = if id == BUILTIN_SPRITE_ATLAS_ID {
                from_rgba8(
                    BUILTIN_ATLAS_WIDTH,
                    BUILTIN_ATLAS_HEIGHT,
                    &builtin_atlas_rgba8(),
                )?
            } else {
                let index = builtin_icon_index(id)?;
                let (_, svg) = builtin::ALL.get(index)?;
                let icon = bake(svg, ICON_PX, ICON_PX).ok()?;
                let rgba: Vec<u8> = icon
                    .alpha
                    .iter()
                    .flat_map(|&a| [255, 255, 255, a])
                    .collect();
                from_rgba8(icon.width, icon.height, &rgba)?
            };
            self.atlases.insert(id, pixmap);
        }
        self.atlases.get(&id)
    }

    pub(super) fn pattern(&mut self, place: &PatternPlacement) -> Option<tiny_skia::Shader<'_>> {
        shader(&mut self.pattern, place)
    }
}

fn place_glyph(g: &RenderedGlyph, place: &RunPlacement) -> (RasterKey, i32, i32) {
    let x = (place.origin.x + g.x).mul_add(place.scale, -place.offset.x);
    let y = (place.origin.y + g.y).mul_add(place.scale, -place.offset.y);
    let (bin, carry) = subpixel_quantize(x);
    (
        RasterKey::new(g, g.size * place.scale, bin),
        (x + carry).floor() as i32,
        y.round() as i32,
    )
}

fn pixel_clip(clip: Option<Rect>, scale: f32, offset: Point, target: &Pixmap) -> [i32; 4] {
    let full = [0, 0, target.width() as i32, target.height() as i32];
    let Some(clip) = clip else {
        return full;
    };
    let x0 = clip.origin.x.mul_add(scale, -offset.x).floor() as i32;
    let y0 = clip.origin.y.mul_add(scale, -offset.y).floor() as i32;
    let x1 = (clip.origin.x + clip.width)
        .mul_add(scale, -offset.x)
        .ceil() as i32;
    let y1 = (clip.origin.y + clip.height)
        .mul_add(scale, -offset.y)
        .ceil() as i32;
    [
        x0.max(full[0]),
        y0.max(full[1]),
        x1.min(full[2]),
        y1.min(full[3]),
    ]
}

fn blit(
    target: &mut Pixmap,
    raster: &GlyphMask,
    (x, y): (i32, i32),
    bounds: [i32; 4],
    table: &[[u8; 4]; 256],
) {
    let stride = target.width() as usize;
    let data = target.data_mut();
    for row in 0..raster.height as i32 {
        let py = y - raster.top + row;
        if py < bounds[1] || py >= bounds[3] {
            continue;
        }
        for col in 0..raster.width as i32 {
            let px = x + raster.left + col;
            if px < bounds[0] || px >= bounds[2] {
                continue;
            }
            let at = (row as u32 * raster.width + col as u32) as usize;
            let cov = usize::from(raster.data.get(at).copied().unwrap_or(0));
            let Some(src) = table.get(cov).filter(|_| cov > 0) else {
                continue;
            };
            let dst_at = (py as usize * stride + px as usize) * 4;
            if let Some(dst) = data
                .get_mut(dst_at..dst_at + 4)
                .and_then(|s| <&mut [u8; 4]>::try_from(s).ok())
            {
                over(dst, *src);
            }
        }
    }
}

fn over(dst: &mut [u8; 4], src: [u8; 4]) {
    let keep = 1.0 - f32::from(src[3]) / 255.0;
    for (d, s) in dst.iter_mut().zip(src).take(3) {
        let linear = decode(*d).mul_add(keep, decode(s));
        *d = (encode(linear) * 255.0).round() as u8;
    }
    dst[3] = f32::from(dst[3]).mul_add(keep, f32::from(src[3])).round() as u8;
}

#[cfg(test)]
#[path = "quad_tests.rs"]
mod tests;
