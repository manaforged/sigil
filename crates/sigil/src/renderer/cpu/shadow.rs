#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::hal::{Point, Rect};
use crate::style::Rgba8;
use tiny_skia::{BlendMode, FillRule, Mask, Paint, Pixmap, PixmapPaint, Transform};

use crate::renderer::corner_radii::CornerRadii;
use crate::renderer::shadow_paint::{Shadow, ShadowKind};

use super::shapes::{encode, rounded_rect, solid, ts_color};

const BLUR_PASSES: u32 = 3;

pub(super) struct ShadowPlacement {
    pub(super) rect: Rect,
    pub(super) radii: CornerRadii,
    pub(super) scale: f32,
    pub(super) offset: Point,
    pub(super) opacity: f32,
}

struct ShadowGeometry {
    surface: Rect,
    radii: CornerRadii,
    body: Rect,
    body_radii: CornerRadii,
    sigma: f32,
    origin: Point,
    width: u32,
    height: u32,
}

impl ShadowGeometry {
    fn new(shadow: &Shadow, place: &ShadowPlacement) -> Self {
        let s = place.scale;
        let surface = Rect::new(
            Point::new(
                place.rect.origin.x.mul_add(s, -place.offset.x),
                place.rect.origin.y.mul_add(s, -place.offset.y),
            ),
            place.rect.width * s,
            place.rect.height * s,
        );
        let radii = CornerRadii(place.radii.0.map(|r| r * s));
        let sigma = shadow.blur.max(0.0) * s;
        let spread = shadow.spread * s;
        let shifted = Rect::new(
            Point::new(
                shadow.offset[0].mul_add(s, surface.origin.x),
                shadow.offset[1].mul_add(s, surface.origin.y),
            ),
            surface.width,
            surface.height,
        );
        let inner = matches!(shadow.kind, ShadowKind::Inner);
        let (body, body_radii) = if inner {
            (
                expand(shifted, -spread),
                CornerRadii(radii.0.map(|r| r - spread)),
            )
        } else {
            (
                expand(shifted, spread),
                CornerRadii(radii.0.map(|r| r + spread)),
            )
        };
        let cover = if inner {
            surface
        } else {
            expand(body, sigma * 3.0)
        };
        let origin = Point::new(cover.origin.x.floor(), cover.origin.y.floor());
        Self {
            surface,
            radii,
            body,
            body_radii,
            sigma,
            origin,
            width: (cover.origin.x + cover.width - origin.x).ceil().max(1.0) as u32,
            height: (cover.origin.y + cover.height - origin.y).ceil().max(1.0) as u32,
        }
    }
}

fn expand(rect: Rect, by: f32) -> Rect {
    Rect::new(
        Point::new(rect.origin.x - by, rect.origin.y - by),
        by.mul_add(2.0, rect.width).max(0.0),
        by.mul_add(2.0, rect.height).max(0.0),
    )
}

fn shadow_color(shadow: &Shadow) -> Rgba8 {
    Rgba8 {
        r: srgb8(encode(shadow.color[0])),
        g: srgb8(encode(shadow.color[1])),
        b: srgb8(encode(shadow.color[2])),
        a: (shadow.color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    }
}

fn scratch(shadow: &Shadow, geo: &ShadowGeometry, opacity: f32) -> Option<Pixmap> {
    let mut scratch = Pixmap::new(geo.width, geo.height)?;
    let color = shadow_color(shadow);
    let local = Transform::from_translate(-geo.origin.x, -geo.origin.y);
    let path = rounded_rect(geo.body, geo.body_radii);
    if matches!(shadow.kind, ShadowKind::Inner) {
        scratch.fill(ts_color(color, opacity));
        let clear = Paint {
            blend_mode: BlendMode::Clear,
            anti_alias: true,
            ..Paint::default()
        };
        if let Some(path) = path {
            scratch.fill_path(&path, &clear, FillRule::Winding, local, None);
        }
    } else if let Some(path) = path {
        scratch.fill_path(
            &path,
            &solid(color, opacity),
            FillRule::Winding,
            local,
            None,
        );
    }
    blur(&mut scratch, geo.sigma);
    Some(scratch)
}

fn surface_mask(target: &Pixmap, geo: &ShadowGeometry) -> Option<Mask> {
    let path = rounded_rect(geo.surface, geo.radii)?;
    let mut mask = Mask::new(target.width(), target.height())?;
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Some(mask)
}

pub(super) fn shadow(
    target: &mut Pixmap,
    shadow: &Shadow,
    place: &ShadowPlacement,
    mask: Option<&Mask>,
) {
    let geo = ShadowGeometry::new(shadow, place);
    let Some(scratch) = scratch(shadow, &geo, place.opacity) else {
        return;
    };
    let inner = matches!(shadow.kind, ShadowKind::Inner);
    let clip = inner.then(|| surface_mask(target, &geo)).flatten();
    let mask = if inner { clip.as_ref() } else { mask };
    target.draw_pixmap(
        geo.origin.x as i32,
        geo.origin.y as i32,
        scratch.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        mask,
    );
}

fn srgb8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(super) fn blur(pixmap: &mut Pixmap, sigma: f32) {
    if sigma < 0.5 {
        return;
    }
    let dims = Dims {
        width: pixmap.width() as usize,
        height: pixmap.height() as usize,
        radius: (sigma * 1.5).round() as usize,
    };
    let mut scratch = vec![[0_u8; 4]; dims.width * dims.height];
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    for _ in 0..BLUR_PASSES {
        box_pass(pixels, &mut scratch, dims, true);
        box_pass(&scratch, pixels, dims, false);
    }
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

fn box_pass(src: &[[u8; 4]], dst: &mut [[u8; 4]], dims: Dims, horizontal: bool) {
    let (lines, len) = if horizontal {
        (dims.height, dims.width)
    } else {
        (dims.width, dims.height)
    };
    let radius = dims.radius;
    let window = (2 * radius + 1) as u32;
    let sample = |line: usize, i: usize| -> [u32; 4] {
        src.get(dims.at(horizontal, line, i))
            .copied()
            .unwrap_or_default()
            .map(u32::from)
    };
    for line in 0..lines {
        let mut sum = [0_u32; 4];
        for i in 0..=radius.min(len.saturating_sub(1)) {
            for (acc, v) in sum.iter_mut().zip(sample(line, i)) {
                *acc += v;
            }
        }
        for i in 0..len {
            if let Some(out) = dst.get_mut(dims.at(horizontal, line, i)) {
                *out = sum.map(|v| (v / window) as u8);
            }
            if i + radius + 1 < len {
                for (acc, v) in sum.iter_mut().zip(sample(line, i + radius + 1)) {
                    *acc += v;
                }
            }
            if i >= radius {
                for (acc, v) in sum.iter_mut().zip(sample(line, i - radius)) {
                    *acc -= v;
                }
            }
        }
    }
}
