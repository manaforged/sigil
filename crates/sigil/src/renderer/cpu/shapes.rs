#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use crate::style::Rgba8;
use tiny_skia::{
    BlendMode, Color as TsColor, ColorSpace, FillRule, FilterQuality, IntSize, Mask, Paint, Path,
    PathBuilder, Pattern, Pixmap, SpreadMode, Stroke as TsStroke,
    Transform as TsTransform,
};

use crate::renderer::cache_limits::CacheLimits;
use crate::renderer::scene_command::{
    DecorationGeometry, DecorationKind, DrawDecoration, Transform,
};

use super::bound::CpuBound;

const WAVE_STEP: f32 = 1.0;
const WAVE_STEPS_PER_PERIOD: f32 = 16.0;

#[derive(Clone, Copy)]
pub(super) struct Brush<'a> {
    pub(super) transform: TsTransform,
    pub(super) mask: Option<&'a Mask>,
    pub(super) cover: Option<&'a Mask>,
    pub(super) opacity: f32,
}

impl<'a> Brush<'a> {
    pub(super) fn mask_for(&self, paint: &Paint<'_>) -> Option<&'a Mask> {
        if paint.shader.is_opaque() {
            self.mask
        } else {
            self.coverage()
        }
    }

    pub(super) fn coverage(&self) -> Option<&'a Mask> {
        self.mask.or(self.cover)
    }
}

pub(super) fn ts_transform(transform: Transform, scale: f32, origin: Point) -> TsTransform {
    let [[sx, kx, tx], [ky, sy, ty]] = transform;
    TsTransform::from_row(sx, ky, kx, sy, tx, ty)
        .post_scale(scale, scale)
        .post_translate(-origin.x, -origin.y)
}

pub(super) fn color(color: crate::style::Color) -> TsColor {
    TsColor::from_rgba(
        encode(color.r),
        encode(color.g),
        encode(color.b),
        color.a.clamp(0.0, 1.0),
    )
    .unwrap_or(TsColor::TRANSPARENT)
}

pub(super) use crate::style::srgb::{decode, encode, encode_byte};

pub(super) fn stored(linear: f32, alpha: f32) -> u8 {
    encode_byte(linear * alpha)
}

pub(super) fn coverage_table(linear: [f32; 3], alpha: f32) -> [[u8; 4]; 256] {
    std::array::from_fn(|coverage| {
        let covered = alpha.clamp(0.0, 1.0) * coverage as f32 / 255.0;
        let [r, g, b] = linear.map(|channel| stored(channel, covered));
        [r, g, b, (covered * 255.0).round() as u8]
    })
}

pub(super) fn rgba_table(color: Rgba8, opacity: f32) -> [[u8; 4]; 256] {
    coverage_table(
        [color.r, color.g, color.b].map(decode),
        f32::from(color.a) / 255.0 * opacity,
    )
}

pub(super) fn ts_color(color: Rgba8, opacity: f32) -> TsColor {
    let alpha = (f32::from(color.a) / 255.0 * opacity).clamp(0.0, 1.0);
    TsColor::from_rgba(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        alpha,
    )
    .unwrap_or(TsColor::TRANSPARENT)
}

pub(super) const BLEND_SPACE: ColorSpace = ColorSpace::Gamma2;

pub(super) fn solid(color: Rgba8, opacity: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    let color = ts_color(color, opacity);
    paint.set_color(color);
    paint.anti_alias = true;
    paint.colorspace = if color.is_opaque() {
        ColorSpace::Linear
    } else {
        BLEND_SPACE
    };
    paint
}

pub(super) fn cover(covers: &mut Vec<Mask>, width: u32, height: u32) -> Option<&Mask> {
    let found = covers
        .iter()
        .position(|mask| mask.width() == width && mask.height() == height);
    if found.is_none() {
        let size = IntSize::from_wh(width, height)?;
        let full = vec![u8::MAX; size.width() as usize * size.height() as usize];
        if covers.len() >= CacheLimits::DEFAULT.cpu_layer_pool {
            let _ = covers.swap_remove(0);
        }
        covers.push(Mask::from_vec(full, size)?);
    }
    covers.get(found.unwrap_or(covers.len() - 1))
}

pub(super) fn blit(
    target: &mut Pixmap,
    source: &Pixmap,
    transform: TsTransform,
    mask: &Mask,
    blend_mode: BlendMode,
) {
    let whole = Rect::new(
        Point::new(0.0, 0.0),
        source.width() as f32,
        source.height() as f32,
    );
    let Some(rect) = CpuBound::of(target, transform).clip_rect(whole) else {
        return;
    };
    let paint = Paint {
        shader: Pattern::new(
            source.as_ref(),
            SpreadMode::Pad,
            FilterQuality::Nearest,
            1.0,
            TsTransform::identity(),
        ),
        anti_alias: false,
        blend_mode,
        colorspace: match blend_mode {
            BlendMode::SourceOver => BLEND_SPACE,
            _ => ColorSpace::Linear,
        },
        ..Paint::default()
    };
    target.fill_rect(rect, &paint, transform, Some(mask));
}

pub(super) fn opacity_of(opacity: u16, layer: f32) -> f32 {
    (f32::from(opacity) / 10_000.0 * layer).clamp(0.0, 1.0)
}

pub(super) fn fill(target: &mut Pixmap, path: &Path, paint: &Paint<'_>, brush: &Brush<'_>) {
    target.fill_path(
        path,
        paint,
        FillRule::Winding,
        brush.transform,
        brush.mask_for(paint),
    );
}

pub(super) fn decoration(target: &mut Pixmap, draw: &DrawDecoration, brush: &Brush<'_>) {
    let Some(geometry) = draw.geometry() else {
        return;
    };
    let paint = solid(draw.color, opacity_of(draw.opacity, brush.opacity));
    let bound = CpuBound::of(target, brush.transform);
    match draw.kind {
        DecorationKind::Underline | DecorationKind::Strikethrough => {
            let Some(rect) = bound.clip_rect(geometry.line()) else {
                return;
            };
            target.fill_rect(rect, &paint, brush.transform, brush.mask_for(&paint));
        }
        DecorationKind::WavyUnderline => {
            let reach = geometry.reach();
            let Some(path) = wave(&geometry, bound.padded(reach)) else {
                return;
            };
            let Some(columns) = band_mask(target, &geometry, brush, &paint) else {
                return;
            };
            let ts = TsStroke {
                width: geometry.thickness,
                ..TsStroke::default()
            };
            target.stroke_path(&path, &paint, &ts, brush.transform, Some(&columns));
        }
    }
}

fn band_mask(
    target: &Pixmap,
    geometry: &DecorationGeometry,
    brush: &Brush<'_>,
    paint: &Paint<'_>,
) -> Option<Mask> {
    let bound = CpuBound::of(target, brush.transform).padded(geometry.reach());
    let band = geometry.band;
    let columns = Rect::new(
        Point::new(band.origin.x, band.origin.y - geometry.reach()),
        band.width,
        geometry.reach().mul_add(2.0, band.height),
    );
    let rect = bound.clip_rect(columns)?;
    let path = PathBuilder::from_rect(rect);
    if let Some(clip) = brush.mask_for(paint) {
        let mut mask = clip.clone();
        mask.intersect_path(&path, FillRule::Winding, true, brush.transform);
        return Some(mask);
    }
    let mut mask = Mask::new(target.width(), target.height())?;
    mask.fill_path(&path, FillRule::Winding, true, brush.transform);
    Some(mask)
}

fn wave(geometry: &DecorationGeometry, bound: CpuBound) -> Option<Path> {
    let band = geometry.band;
    let reach = geometry.reach();
    let span = Rect::new(
        Point::new(band.origin.x - reach, band.origin.y),
        reach.mul_add(2.0, band.width),
        band.height,
    );
    let visible = bound.clip(span)?;
    let step = WAVE_STEP.min(geometry.period / WAVE_STEPS_PER_PERIOD);
    let lo = visible.origin.x - band.origin.x;
    let first = (lo / step).floor() as i64;
    let last = ((lo + visible.width) / step).ceil() as i64;
    let mut b = PathBuilder::new();
    for index in first..=last {
        let x = (index as f32 * step).clamp(-reach, band.width + reach);
        let y = geometry.wave_y(x);
        if index == first {
            b.move_to(band.origin.x + x, y);
        } else {
            b.line_to(band.origin.x + x, y);
        }
    }
    b.finish()
}
