#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "region texels are clamped to the atlas before the cast"
)]

use crate::geometry::Rect;
use tiny_skia::Pixmap;

use crate::renderer::scene_command::{DrawSprite, SpriteKind};

use super::shapes::{coverage_table, decode, stored};

const SUBPIXEL_TAPS: [f32; 3] = [-1.0 / 3.0, 0.0, 1.0 / 3.0];

struct Texels<'a> {
    pixels: &'a [[u8; 4]],
    stride: usize,
    left: usize,
    top: usize,
    width: usize,
}

impl Texels<'_> {
    fn at(&self, row: usize, col: usize) -> [u8; 4] {
        self.pixels
            .get((self.top + row) * self.stride + self.left + col)
            .copied()
            .unwrap_or_default()
    }

    fn alpha_at(&self, row: usize, x: f32) -> f32 {
        let last = self.width.saturating_sub(1) as f32;
        let x = (x - 0.5).clamp(0.0, last);
        let lo = x.floor();
        let hi = (lo + 1.0).min(last);
        let a = f32::from(self.at(row, lo as usize)[3]);
        let b = f32::from(self.at(row, hi as usize)[3]);
        (b - a).mul_add(x - lo, a) / 255.0
    }
}

pub(super) fn tint(atlas: &Pixmap, region: Rect, draw: &DrawSprite) -> Option<Pixmap> {
    let left = region.origin.x.max(0.0) as u32;
    let top = region.origin.y.max(0.0) as u32;
    let width = (region.width.round() as u32)
        .min(atlas.width().saturating_sub(left))
        .max(1);
    let height = (region.height.round() as u32)
        .min(atlas.height().saturating_sub(top))
        .max(1);
    let mut out = Pixmap::new(width, height)?;
    let (pixels, _) = atlas.data().as_chunks::<4>();
    let texels = Texels {
        pixels,
        stride: atlas.width() as usize,
        left: left as usize,
        top: top as usize,
        width: width as usize,
    };
    let c = draw.color;
    let table = coverage_table([c.r, c.g, c.b].map(decode), f32::from(c.a) / 255.0);
    let row_len = width as usize;
    let (dst, _) = out.data_mut().as_chunks_mut::<4>();
    for (index, px) in dst.iter_mut().enumerate() {
        let (row, col) = (index / row_len, index % row_len);
        *px = match draw.kind {
            SpriteKind::Poly => texels.at(row, col),
            SpriteKind::Mono => table
                .get(usize::from(texels.at(row, col)[3]))
                .copied()
                .unwrap_or_default(),
            SpriteKind::Subpixel => subpixel(draw, &texels, row, col),
        };
    }
    Some(out)
}

fn subpixel(draw: &DrawSprite, texels: &Texels<'_>, row: usize, col: usize) -> [u8; 4] {
    let center = col as f32 + 0.5 + draw.source.phase;
    let [r, g, b] = SUBPIXEL_TAPS.map(|tap| texels.alpha_at(row, center + tap));
    let c = draw.color;
    let alpha = (f32::from(c.a) / 255.0 * r.max(g).max(b)).clamp(0.0, 1.0);
    let [lr, lg, lb] = [c.r, c.g, c.b].map(decode);
    [
        stored(lr * r, alpha),
        stored(lg * g, alpha),
        stored(lb * b, alpha),
        (alpha * 255.0).round() as u8,
    ]
}
