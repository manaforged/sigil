#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::hal::{Point, Rect, RenderedGlyph};
use crate::renderer::oblique::Oblique;
use ahash::AHashMap;
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider as _};
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PremultipliedColorU8, Transform};

use crate::renderer::scene_command::GlyphRun;

const SUBPIXEL_STEPS: f32 = 4.0;
const SIZE_UNIT: f32 = 64.0;
const MAX_GLYPHS: usize = 8_192;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct Key {
    font: u64,
    index: u32,
    glyph: u32,
    size: u32,
    subpixel: u8,
    coords: u64,
}

struct Raster {
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    alpha: Vec<u8>,
}

#[derive(Default)]
pub(super) struct GlyphCache {
    masks: AHashMap<Key, Option<Raster>>,
}

pub(super) struct RunPlacement {
    pub(super) origin: Point,
    pub(super) scale: f32,
    pub(super) offset: Point,
    pub(super) clip: Option<Rect>,
    pub(super) color: PremultipliedColorU8,
}

impl GlyphCache {
    pub(super) fn draw_run(&mut self, target: &mut Pixmap, run: &GlyphRun, place: &RunPlacement) {
        let Some(paragraph) = run.paragraph() else {
            return;
        };
        let glyphs = run.positioned_glyphs();
        let placed: Vec<(Key, i32, i32)> = glyphs
            .iter()
            .map(|g| {
                let x = (place.origin.x + g.x).mul_add(place.scale, -place.offset.x);
                let y = (place.origin.y + g.y).mul_add(place.scale, -place.offset.y);
                let subpixel = ((x.fract() * SUBPIXEL_STEPS).round() as u8) % SUBPIXEL_STEPS as u8;
                (
                    key(g, place.scale, subpixel),
                    x.floor() as i32,
                    y.floor() as i32,
                )
            })
            .collect();
        if placed.iter().any(|(k, _, _)| !self.masks.contains_key(k)) {
            if self.masks.len() >= MAX_GLYPHS {
                self.masks.clear();
            }
            paragraph.for_each_font(|font_id, bytes, font_index| {
                for ((k, _, _), g) in placed.iter().zip(glyphs) {
                    if k.font == font_id && k.index == font_index && !self.masks.contains_key(k) {
                        self.masks.insert(*k, rasterize(bytes, font_index, g, k));
                    }
                }
            });
        }
        let bounds = pixel_clip(place.clip, place.scale, place.offset, target);
        for (k, x, y) in placed {
            if let Some(Some(raster)) = self.masks.get(&k) {
                blit(
                    target,
                    raster,
                    Blit {
                        x,
                        y,
                        bounds,
                        color: place.color,
                    },
                );
            }
        }
    }
}

fn key(g: &RenderedGlyph, scale: f32, subpixel: u8) -> Key {
    Key {
        font: g.font_id,
        index: g.font_index,
        glyph: g.glyph_id,
        size: (g.size * scale * SIZE_UNIT).round() as u32,
        subpixel,
        coords: ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&g.variation_coords),
    }
}

struct Pen(PathBuilder);

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, -y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, -y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to(cx, -cy, x, -y);
    }
    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        self.0.cubic_to(c0x, -c0y, c1x, -c1y, x, -y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

fn rasterize(bytes: &[u8], font_index: u32, g: &RenderedGlyph, k: &Key) -> Option<Raster> {
    let font = FontRef::from_index(bytes, font_index).ok()?;
    let outline = font.outline_glyphs().get(GlyphId::new(g.glyph_id))?;
    let coords: Vec<NormalizedCoord> = g
        .variation_coords
        .iter()
        .map(|&c| NormalizedCoord::from_bits(c))
        .collect();
    let px = k.size as f32 / SIZE_UNIT;
    let settings = DrawSettings::unhinted(Size::new(px), LocationRef::new(&coords));
    let mut pen = Pen(PathBuilder::new());
    outline
        .draw(settings, &mut Oblique::new(&mut pen, g.skew))
        .ok()?;
    let shift = f32::from(k.subpixel) / SUBPIXEL_STEPS;
    let path = pen
        .0
        .finish()?
        .transform(Transform::from_translate(shift, 0.0))?;
    let b = path.bounds();
    let (left, top) = (b.left().floor() as i32, b.top().floor() as i32);
    let width = (b.right().ceil() as i32 - left).max(1) as u32;
    let height = (b.bottom().ceil() as i32 - top).max(1) as u32;
    let mut mask = Mask::new(width, height)?;
    mask.fill_path(
        &path,
        FillRule::Winding,
        true,
        Transform::from_translate(-(left as f32), -(top as f32)),
    );
    Some(Raster {
        left,
        top,
        width,
        height,
        alpha: mask.data().to_vec(),
    })
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

#[derive(Clone, Copy)]
struct Blit {
    x: i32,
    y: i32,
    bounds: [i32; 4],
    color: PremultipliedColorU8,
}

fn blit(target: &mut Pixmap, raster: &Raster, blit: Blit) {
    let stride = target.width() as usize;
    let src = [
        blit.color.red(),
        blit.color.green(),
        blit.color.blue(),
        blit.color.alpha(),
    ]
    .map(u32::from);
    let data = target.data_mut();
    for row in 0..raster.height as i32 {
        let py = blit.y + raster.top + row;
        if py < blit.bounds[1] || py >= blit.bounds[3] {
            continue;
        }
        for col in 0..raster.width as i32 {
            let px = blit.x + raster.left + col;
            if px < blit.bounds[0] || px >= blit.bounds[2] {
                continue;
            }
            let at = (row as u32 * raster.width + col as u32) as usize;
            let cov = u32::from(raster.alpha.get(at).copied().unwrap_or(0));
            if cov == 0 {
                continue;
            }
            let dst_at = (py as usize * stride + px as usize) * 4;
            let Some(dst) = data
                .get_mut(dst_at..dst_at + 4)
                .and_then(|s| <&mut [u8; 4]>::try_from(s).ok())
            else {
                continue;
            };
            let inv = 255 - (src[3] * cov + 127) / 255;
            for (d, s) in dst.iter_mut().zip(src) {
                *d = ((s * cov + u32::from(*d) * inv + 127) / 255) as u8;
            }
        }
    }
}
