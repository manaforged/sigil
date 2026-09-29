#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use crate::hal::RenderedGlyph;
use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::raster::{GlyphMask, Hinters, RasterKey, subpixel_quantize};
use crate::style::Rgba8;
use ahash::AHashMap;
use tiny_skia::Pixmap;

use super::shapes::{coverage_table, decode, encode};

use crate::renderer::scene_command::GlyphRun;

#[derive(Default)]
pub(super) struct GlyphCache {
    masks: AHashMap<RasterKey, (Option<GlyphMask>, u64)>,
    hinters: Hinters,
    frame: u64,
}

pub(super) struct RunPlacement {
    pub(super) origin: Point,
    pub(super) scale: f32,
    pub(super) offset: Point,
    pub(super) clip: Option<Rect>,
    pub(super) table: [[u8; 4]; 256],
}

pub(super) fn glyph_table(color: Rgba8, opacity: f32) -> [[u8; 4]; 256] {
    coverage_table(
        [color.r, color.g, color.b].map(decode),
        f32::from(color.a) / 255.0 * opacity,
    )
}

impl GlyphCache {
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
        let limits = CacheLimits::DEFAULT;
        if self.masks.len() > limits.glyph_masks {
            let frame = self.frame;
            self.masks
                .retain(|_, (_, used)| *used + limits.glyph_stale_frames >= frame);
        }
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
                    blit(
                        target,
                        mask,
                        Blit {
                            x,
                            y,
                            bounds,
                            table: &place.table,
                        },
                    );
                }
            }
        }
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

#[derive(Clone, Copy)]
struct Blit<'a> {
    x: i32,
    y: i32,
    bounds: [i32; 4],
    table: &'a [[u8; 4]; 256],
}

fn blit(target: &mut Pixmap, raster: &GlyphMask, blit: Blit<'_>) {
    let stride = target.width() as usize;
    let data = target.data_mut();
    for row in 0..raster.height as i32 {
        let py = blit.y - raster.top + row;
        if py < blit.bounds[1] || py >= blit.bounds[3] {
            continue;
        }
        for col in 0..raster.width as i32 {
            let px = blit.x + raster.left + col;
            if px < blit.bounds[0] || px >= blit.bounds[2] {
                continue;
            }
            let at = (row as u32 * raster.width + col as u32) as usize;
            let cov = usize::from(raster.data.get(at).copied().unwrap_or(0));
            let Some(src) = blit.table.get(cov).filter(|_| cov > 0) else {
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
#[path = "glyphs_tests.rs"]
mod tests;
