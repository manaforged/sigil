use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, pen::PathElement},
};

use crate::text::outline::{Oblique, coords};

use super::curve::{QuadBez, cubic_to_quads, line_to_quad};

pub(super) const BANDS: usize = 16;
pub(super) const BAND_HEADER_FLOATS: usize = 2;
pub(super) const HEADER_FLOATS: usize = 2 * BANDS * BAND_HEADER_FLOATS;

#[derive(Copy, Clone, Debug)]
pub(super) struct GlyphBbox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl GlyphBbox {
    pub(super) fn width(self) -> f32 {
        self.x_max - self.x_min
    }
    pub(super) fn height(self) -> f32 {
        self.y_max - self.y_min
    }
}

#[derive(Clone)]
pub(super) struct BandData {
    pub data: Vec<f32>,
    pub bbox: GlyphBbox,
    pub units_per_em: u16,
}

impl BandData {
    pub(super) fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }
}

pub(super) fn preprocess_glyph(
    font_bytes: &[u8],
    font_index: u32,
    glyph_id: u32,
    variation_coords: &[i16],
    skew: f32,
) -> Option<BandData> {
    let font = FontRef::from_index(font_bytes, font_index).ok()?;
    let upem = font
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em;
    let glyph = font.outline_glyphs().get(GlyphId::new(glyph_id))?;
    let coords = coords(variation_coords);
    let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::new(&coords));
    let mut elements: Vec<PathElement> = Vec::new();
    glyph
        .draw(settings, &mut Oblique::new(&mut elements, skew))
        .ok()?;
    let quads = path_elements_to_quads(&elements);
    if quads.is_empty() {
        return None;
    }
    let bbox = compute_bbox(&quads)?;
    let (w, h) = (bbox.width(), bbox.height());
    if w < 1e-4 || h < 1e-4 {
        return None;
    }
    let n = |pt: [f32; 2]| [(pt[0] - bbox.x_min) / w, 1.0 - (pt[1] - bbox.y_min) / h];
    let normalised: Vec<QuadBez> = quads
        .iter()
        .map(|q| QuadBez {
            p0: n(q.p0),
            p1: n(q.p1),
            p2: n(q.p2),
        })
        .collect();
    Some(pack_into_bands(&normalised, bbox, upem))
}

fn path_elements_to_quads(elements: &[PathElement]) -> Vec<QuadBez> {
    let mut quads = Vec::new();
    let mut cur = [0.0f32; 2];
    let mut start = [0.0f32; 2];
    for &elem in elements {
        match elem {
            PathElement::MoveTo { x, y } => {
                cur = [x, y];
                start = [x, y];
            }
            PathElement::LineTo { x, y } => {
                quads.push(line_to_quad(cur, [x, y]));
                cur = [x, y];
            }
            PathElement::QuadTo { cx0, cy0, x, y } => {
                quads.push(QuadBez {
                    p0: cur,
                    p1: [cx0, cy0],
                    p2: [x, y],
                });
                cur = [x, y];
            }
            PathElement::CurveTo {
                cx0,
                cy0,
                cx1,
                cy1,
                x,
                y,
            } => {
                let [q0, q1] = cubic_to_quads(cur, [cx0, cy0], [cx1, cy1], [x, y]);
                quads.push(q0);
                quads.push(q1);
                cur = [x, y];
            }
            PathElement::Close => {
                let dx = (cur[0] - start[0]).abs();
                let dy = (cur[1] - start[1]).abs();
                if dx > 1e-4 || dy > 1e-4 {
                    quads.push(line_to_quad(cur, start));
                }
                cur = start;
            }
        }
    }
    quads
}

fn compute_bbox(quads: &[QuadBez]) -> Option<GlyphBbox> {
    let mut x_min = f32::MAX;
    let mut y_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_max = f32::MIN;
    for q in quads {
        for pt in [q.p0, q.p1, q.p2] {
            x_min = x_min.min(pt[0]);
            y_min = y_min.min(pt[1]);
            x_max = x_max.max(pt[0]);
            y_max = y_max.max(pt[1]);
        }
    }
    (x_min <= x_max).then_some(GlyphBbox {
        x_min,
        y_min,
        x_max,
        y_max,
    })
}

fn pack_into_bands(quads: &[QuadBez], bbox: GlyphBbox, units_per_em: u16) -> BandData {
    let axes = [
        partition_bands(quads, |p| p[1]),
        partition_bands(quads, |p| p[0]),
    ];
    let mut data: Vec<f32> = Vec::with_capacity(HEADER_FLOATS);
    let mut curves: Vec<f32> = Vec::new();
    for band in axes.iter().flatten() {
        let entry: [f32; BAND_HEADER_FLOATS] = [
            exact_f32(HEADER_FLOATS + curves.len()),
            exact_f32(band.len()),
        ];
        data.extend(entry);
        curves.extend(band.iter().flat_map(|quad| quad.floats()));
    }
    data.append(&mut curves);
    BandData {
        data,
        bbox,
        units_per_em,
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "glyph float counts stay far below 2^24, where f32 holds every integer exactly"
)]
const fn exact_f32(n: usize) -> f32 {
    n as f32
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn partition_bands(quads: &[QuadBez], axis: fn([f32; 2]) -> f32) -> [Vec<QuadBez>; BANDS] {
    let mut bands: [Vec<QuadBez>; BANDS] = core::array::from_fn(|_| Vec::new());
    for quad in quads {
        let [a, b, c] = [axis(quad.p0), axis(quad.p1), axis(quad.p2)];
        let (lo, hi) = (a.min(b).min(c), a.max(b).max(c));
        let b_lo = ((lo * BANDS as f32).floor() as usize).min(BANDS - 1);
        let b_hi = ((hi * BANDS as f32).ceil() as usize).min(BANDS);
        for slot in b_lo..b_hi {
            if let Some(band) = bands.get_mut(slot) {
                band.push(*quad);
            }
        }
    }
    bands
}
