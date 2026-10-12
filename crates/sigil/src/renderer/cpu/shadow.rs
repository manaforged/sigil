#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{Mask, Pixmap, Transform};

use crate::renderer::blur_paint::{BLUR_PASSES, box_radius};
use crate::renderer::scene_command::walk::intersect_rect;
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::expanded_shadow_rect;
use crate::style::srgb;

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
    );
}

pub(super) fn blur(pixmap: &mut Pixmap, sigma: f32) {
    let radius = box_radius(sigma) as usize;
    if radius == 0 {
        return;
    }
    let dims = Dims {
        width: pixmap.width() as usize,
        height: pixmap.height() as usize,
        radius,
    };
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    let mut light: Vec<[f32; 4]> = pixels.iter().map(|p| to_linear(*p)).collect();
    let mut scratch = vec![[0.0_f32; 4]; light.len()];
    for _ in 0..BLUR_PASSES {
        box_pass(&light, &mut scratch, dims, true);
        box_pass(&scratch, &mut light, dims, false);
    }
    for (pixel, value) in pixels.iter_mut().zip(&light) {
        *pixel = to_encoded(*value);
    }
}

fn to_linear([r, g, b, a]: [u8; 4]) -> [f32; 4] {
    let [r, g, b] = [r, g, b].map(srgb::decode);
    [r, g, b, f32::from(a) / 255.0]
}

fn to_encoded([r, g, b, a]: [f32; 4]) -> [u8; 4] {
    let byte = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    let [r, g, b] = [r, g, b].map(|v| byte(srgb::encode(v)));
    [r, g, b, byte(a)]
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
