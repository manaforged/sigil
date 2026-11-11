#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{Mask, Pixmap, Transform};

use crate::renderer::blur_paint::{BLUR_PASSES, box_radius, downsample_levels};
use crate::renderer::scene_command::walk::intersect_rect;
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::expanded_shadow_rect;
use crate::style::srgb::{decode, decode_unit, encode, encode_byte, unit};

use super::shadow_field::ShadowField;
use super::shapes::{blit, coverage_table};

pub(super) struct ShadowPlacement {
    pub(super) rect: Rect,
    pub(super) radii: CornerRadii,
    pub(super) scale: f32,
    pub(super) offset: Point,
    pub(super) opacity: f32,
}

fn field(shadow: &Shadow, place: &ShadowPlacement) -> ShadowField {
    let s = place.scale;
    let rect = place.rect;
    ShadowField {
        center: (
            rect.width
                .mul_add(0.5, rect.origin.x)
                .mul_add(s, -place.offset.x),
            rect.height
                .mul_add(0.5, rect.origin.y)
                .mul_add(s, -place.offset.y),
        ),
        half: (
            (rect.width * 0.5 * s).max(0.0),
            (rect.height * 0.5 * s).max(0.0),
        ),
        radii: place
            .radii
            .normalized_for(rect.width, rect.height)
            .map(|r| r * s),
        offset: (shadow.offset[0] * s, shadow.offset[1] * s),
        sigma: shadow.blur.max(0.0) * s,
        spread: shadow.spread * s,
        kind: shadow.kind,
        unit: s,
    }
}

fn cover(shadow: &Shadow, place: &ShadowPlacement, target: &Pixmap) -> Option<Rect> {
    let s = place.scale;
    let logical = expanded_shadow_rect(place.rect, *shadow);
    let device = Rect::new(
        Point::new(
            logical.origin.x.mul_add(s, -place.offset.x),
            logical.origin.y.mul_add(s, -place.offset.y),
        ),
        logical.width * s,
        logical.height * s,
    );
    let view = Rect::new(
        Point::new(0.0, 0.0),
        target.width() as f32,
        target.height() as f32,
    );
    intersect_rect(device, view)
}

fn scratch(shadow: &Shadow, place: &ShadowPlacement, cover: Rect) -> Option<(Pixmap, Point)> {
    let origin = Point::new(cover.origin.x.floor(), cover.origin.y.floor());
    let width = (cover.origin.x + cover.width - origin.x).ceil().max(1.0) as u32;
    let height = (cover.origin.y + cover.height - origin.y).ceil().max(1.0) as u32;
    let mut pixmap = Pixmap::new(width, height)?;
    let eval = field(shadow, place);
    let table = coverage_table(
        [shadow.color[0], shadow.color[1], shadow.color[2]],
        shadow.color[3] * place.opacity,
    );
    let row = width as usize;
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    for (index, px) in pixels.iter_mut().enumerate() {
        let x = origin.x + (index % row) as f32 + 0.5;
        let y = origin.y + (index / row) as f32 + 0.5;
        let value = (eval.coverage(x, y) * 255.0).round().clamp(0.0, 255.0) as usize;
        if let Some(tinted) = table.get(value) {
            *px = *tinted;
        }
    }
    Some((pixmap, origin))
}

pub(super) fn shadow(target: &mut Pixmap, shadow: &Shadow, place: &ShadowPlacement, mask: &Mask) {
    let Some(area) = cover(shadow, place, target) else {
        return;
    };
    let Some((scratch, origin)) = scratch(shadow, place, area) else {
        return;
    };
    blit(
        target,
        &scratch,
        Transform::from_translate(origin.x, origin.y),
        mask,
        tiny_skia::BlendMode::SourceOver,
    );
}

pub(super) fn blur(pixmap: &mut Pixmap, sigma: f32) {
    let levels = downsample_levels(sigma);
    let factor = 1_u32 << levels;
    let radius = box_radius(sigma / factor as f32) as usize;
    if radius == 0 {
        return;
    }
    let full = Dims {
        width: pixmap.width() as usize,
        height: pixmap.height() as usize,
        radius,
    };
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    let mut light: Vec<[f32; 4]> = pixels.iter().map(|p| to_premultiplied(*p)).collect();
    let mut dims = full;
    for _ in 0..levels {
        (light, dims) = halve(&light, dims);
    }
    let mut scratch = vec![[0.0_f32; 4]; light.len()];
    for _ in 0..BLUR_PASSES {
        box_pass(&light, &mut scratch, dims, true);
        box_pass(&scratch, &mut light, dims, false);
    }
    if levels > 0 {
        light = upsample(&light, dims, full, factor as f32);
    }
    for (pixel, value) in pixels.iter_mut().zip(&light) {
        *pixel = to_encoded(*value);
    }
}

fn texel(src: &[[f32; 4]], dims: Dims, x: i64, y: i64) -> [f32; 4] {
    let inside = (0..dims.width as i64).contains(&x) && (0..dims.height as i64).contains(&y);
    if !inside {
        return [0.0; 4];
    }
    src.get(dims.at(true, y as usize, x as usize))
        .copied()
        .unwrap_or_default()
}

fn halve(src: &[[f32; 4]], dims: Dims) -> (Vec<[f32; 4]>, Dims) {
    let half = Dims {
        width: dims.width.div_ceil(2),
        height: dims.height.div_ceil(2),
        ..dims
    };
    let out = (0..half.width * half.height)
        .map(|index| {
            let (x, y) = (
                (index % half.width) as i64 * 2,
                (index / half.width) as i64 * 2,
            );
            let mut sum = [0.0_f32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                for (acc, v) in sum.iter_mut().zip(texel(src, dims, x + dx, y + dy)) {
                    *acc += v;
                }
            }
            sum.map(|v| v * 0.25)
        })
        .collect();
    (out, half)
}

fn upsample(src: &[[f32; 4]], dims: Dims, full: Dims, factor: f32) -> Vec<[f32; 4]> {
    let lerp = |a: [f32; 4], b: [f32; 4], t: f32| {
        let mut out = a;
        for (o, v) in out.iter_mut().zip(b) {
            *o = (v - *o).mul_add(t, *o);
        }
        out
    };
    (0..full.width * full.height)
        .map(|index| {
            let px = ((index % full.width) as f32 + 0.5) / factor - 0.5;
            let py = ((index / full.width) as f32 + 0.5) / factor - 0.5;
            let (x, y) = (px.floor() as i64, py.floor() as i64);
            let (tx, ty) = (px - px.floor(), py - py.floor());
            let top = lerp(texel(src, dims, x, y), texel(src, dims, x + 1, y), tx);
            let bottom = lerp(
                texel(src, dims, x, y + 1),
                texel(src, dims, x + 1, y + 1),
                tx,
            );
            lerp(top, bottom, ty)
        })
        .collect()
}

fn to_premultiplied(pixel: [u8; 4]) -> [f32; 4] {
    let alpha = unit(pixel[3]);
    if alpha <= 0.0 {
        return [0.0; 4];
    }
    let channel = |stored: u8| encode((decode(stored) / alpha).min(1.0)) * alpha;
    [
        channel(pixel[0]),
        channel(pixel[1]),
        channel(pixel[2]),
        alpha,
    ]
}

fn to_encoded(value: [f32; 4]) -> [u8; 4] {
    let alpha = value[3].clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return [0; 4];
    }
    let channel = |srgb: f32| encode_byte(decode_unit((srgb / alpha).min(1.0)) * alpha);
    [
        channel(value[0]),
        channel(value[1]),
        channel(value[2]),
        (alpha * 255.0).round() as u8,
    ]
}

#[derive(Clone, Copy)]
struct Dims {
    width: usize,
    height: usize,
    radius: usize,
}

impl Dims {
    const fn at(self, horizontal: bool, line: usize, i: usize) -> usize {
        if horizontal {
            line * self.width + i
        } else {
            i * self.width + line
        }
    }
}

fn box_pass(src: &[[f32; 4]], dst: &mut [[f32; 4]], dims: Dims, horizontal: bool) {
    let (lines, len) = if horizontal {
        (dims.height, dims.width)
    } else {
        (dims.width, dims.height)
    };
    let radius = dims.radius;
    let window = (2 * radius + 1) as f32;
    let add = |sum: &mut [f32; 4], line: usize, i: usize, subtract: bool| {
        let sample = src
            .get(dims.at(horizontal, line, i))
            .copied()
            .unwrap_or_default();
        for (acc, v) in sum.iter_mut().zip(sample) {
            if subtract {
                *acc -= v;
            } else {
                *acc += v;
            }
        }
    };
    for line in 0..lines {
        let mut sum = [0.0_f32; 4];
        for i in 0..=radius.min(len.saturating_sub(1)) {
            add(&mut sum, line, i, false);
        }
        for i in 0..len {
            if let Some(out) = dst.get_mut(dims.at(horizontal, line, i)) {
                *out = sum.map(|v| v / window);
            }
            if i + radius + 1 < len {
                add(&mut sum, line, i + radius + 1, false);
            }
            if i >= radius {
                add(&mut sum, line, i - radius, true);
            }
        }
    }
}
