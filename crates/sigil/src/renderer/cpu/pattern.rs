#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "device spans are clamped to the target size before the float-to-int conversion"
)]

use crate::hal::Rect;
use crate::style::Rgba8;
use tiny_skia::{FilterQuality, Pattern, Pixmap, Point as TsPoint, Shader, SpreadMode, Transform};

use crate::renderer::scene_command::PatternKind;

use super::bound::Bound;
use super::shapes::{coverage_table, decode};

pub(super) struct Placement {
    pub(super) kind: PatternKind,
    pub(super) rect: Rect,
    pub(super) transform: Transform,
    pub(super) opacity: f32,
    pub(super) bound: Bound,
    pub(super) size: [u32; 2],
}

pub(super) fn shader<'a>(slot: &'a mut Option<Pixmap>, place: &Placement) -> Option<Shader<'a>> {
    let visible = place.bound.clip(place.rect)?;
    let [x0, y0, x1, y1] = device_span(visible, place.transform, place.size)?;
    let inverse = place.transform.invert()?;
    let pixmap = scratch(slot, x1 - x0, y1 - y0)?;
    let field = Field::of(place, inverse);
    paint_field(pixmap, &field, inverse, [x0, y0]);
    let pixmap: &'a Pixmap = pixmap;
    Some(Pattern::new(
        pixmap.as_ref(),
        SpreadMode::Pad,
        FilterQuality::Nearest,
        place.opacity,
        inverse.pre_translate(x0 as f32, y0 as f32),
    ))
}

fn device_span(local: Rect, transform: Transform, [width, height]: [u32; 2]) -> Option<[u32; 4]> {
    let (x0, y0) = (local.origin.x, local.origin.y);
    let (x1, y1) = (x0 + local.width, y0 + local.height);
    let mut points = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| TsPoint::from_xy(x, y));
    transform.map_points(&mut points);
    let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in points {
        (lo_x, lo_y) = (lo_x.min(p.x), lo_y.min(p.y));
        (hi_x, hi_y) = (hi_x.max(p.x), hi_y.max(p.y));
    }
    let (sx, ex) = span(lo_x, hi_x, width)?;
    let (sy, ey) = span(lo_y, hi_y, height)?;
    Some([sx, sy, ex, ey])
}

fn span(lo: f32, hi: f32, limit: u32) -> Option<(u32, u32)> {
    let limit = limit as f32;
    let a = lo.floor().clamp(0.0, limit);
    let b = hi.ceil().clamp(0.0, limit);
    (a.is_finite() && b.is_finite() && b > a).then_some((a as u32, b as u32))
}

fn scratch(slot: &mut Option<Pixmap>, width: u32, height: u32) -> Option<&mut Pixmap> {
    if slot
        .as_ref()
        .is_none_or(|p| p.width() != width || p.height() != height)
    {
        *slot = Some(Pixmap::new(width, height)?);
    }
    slot.as_mut()
}

fn paint_field(pixmap: &mut Pixmap, field: &Field, inverse: Transform, [x0, y0]: [u32; 2]) {
    let row = pixmap.width() as usize;
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let mut p = TsPoint::from_xy(
            (x0 as usize + index % row) as f32 + 0.5,
            (y0 as usize + index / row) as f32 + 0.5,
        );
        inverse.map_point(&mut p);
        *pixel = field.sample(p);
    }
}

enum Look {
    Checker {
        a: [u8; 4],
        b: [u8; 4],
        cell: f64,
    },
    Slash {
        table: Box<[[u8; 4]; 256]>,
        period: f64,
        line: f64,
        aa: f64,
    },
}

struct Field {
    look: Look,
    origin: [f64; 2],
    tile: f64,
}

impl Field {
    fn of(place: &Placement, inverse: Transform) -> Self {
        let kind = place.kind.normalized();
        let look = match kind {
            PatternKind::Checker {
                color_a,
                color_b,
                cell,
            } => Look::Checker {
                a: full(color_a),
                b: full(color_b),
                cell: f64::from(cell),
            },
            PatternKind::Slash {
                color,
                period,
                line_width,
            } => Look::Slash {
                table: Box::new(table(color)),
                period: f64::from(period),
                line: f64::from(line_width),
                aa: f64::from((inverse.sx - inverse.ky).abs() + (inverse.kx - inverse.sy).abs())
                    * 0.5,
            },
        };
        Self {
            look,
            origin: [
                f64::from(place.rect.origin.x),
                f64::from(place.rect.origin.y),
            ],
            tile: f64::from(kind.tile_extent()),
        }
    }

    fn sample(&self, p: TsPoint) -> [u8; 4] {
        let [ox, oy] = self.origin;
        let u = (f64::from(p.x) - ox).rem_euclid(self.tile);
        let v = (f64::from(p.y) - oy).rem_euclid(self.tile);
        match &self.look {
            Look::Checker { a, b, cell } => {
                let parity = ((u / cell).floor() + (v / cell).floor()).rem_euclid(2.0);
                if parity < 1.0 { *a } else { *b }
            }
            Look::Slash {
                table,
                period,
                line,
                aa,
            } => {
                let mask = slash_mask(u - v, *period, *line, *aa);
                let level = (mask * 255.0).round().clamp(0.0, 255.0) as usize;
                table.get(level).copied().unwrap_or_default()
            }
        }
    }
}

fn slash_mask(diagonal: f64, period: f64, line: f64, aa: f64) -> f64 {
    if line <= 0.0 {
        return 0.0;
    }
    if line >= period {
        return 1.0;
    }
    let turns = diagonal / period;
    let phase = (turns - turns.floor()) * period;
    let edge = line.mul_add(-0.5, phase.min(period - phase));
    1.0 - smoothstep(-aa, aa, edge)
}

fn smoothstep(lo: f64, hi: f64, x: f64) -> f64 {
    if hi <= lo {
        return if x < lo { 0.0 } else { 1.0 };
    }
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * 2.0f64.mul_add(-t, 3.0)
}

fn table(color: Rgba8) -> [[u8; 4]; 256] {
    coverage_table(
        [decode(color.r), decode(color.g), decode(color.b)],
        f32::from(color.a) / 255.0,
    )
}

fn full(color: Rgba8) -> [u8; 4] {
    table(color).last().copied().unwrap_or_default()
}
