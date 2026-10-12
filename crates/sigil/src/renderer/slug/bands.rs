use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, pen::PathElement},
};

use crate::renderer::oblique::Oblique;

use super::curve::{QuadBez, cubic_to_quads, line_to_quad};

pub(super) const BANDS: usize = 16;
pub(super) const MAX_CURVES_PER_BAND: usize = 32;
pub(super) const FLOATS_PER_BAND: usize = 1 + MAX_CURVES_PER_BAND * 6;
pub(super) const FLOATS_PER_AXIS: usize = BANDS * FLOATS_PER_BAND;
pub(super) const FLOATS_TOTAL: usize = 2 * FLOATS_PER_AXIS;

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
    let coords: Vec<NormalizedCoord> = variation_coords
        .iter()
        .map(|&c| NormalizedCoord::from_bits(c))
        .collect();
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
    let mut data = vec![0.0f32; FLOATS_TOTAL];
    let (h_out, v_out) = data.split_at_mut(FLOATS_PER_AXIS);
    pack_bands(&partition_bands(quads, |p| p[1]), h_out);
    pack_bands(&partition_bands(quads, |p| p[0]), v_out);
    BandData {
        data,
        bbox,
        units_per_em,
    }
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
    #[cfg(debug_assertions)]
    warn_band_overflow(&bands);
    bands
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn pack_bands(band_curves: &[Vec<QuadBez>; BANDS], out: &mut [f32]) {
    for (chunk, band) in out
        .as_chunks_mut::<FLOATS_PER_BAND>()
        .0
        .iter_mut()
        .zip(band_curves.iter())
    {
        if let Some((count_slot, curves_data)) = chunk.split_first_mut() {
            *count_slot = band.len().min(MAX_CURVES_PER_BAND) as f32;
            for (cf, quad) in curves_data
                .as_chunks_mut::<6>()
                .0
                .iter_mut()
                .zip(band.iter())
            {
                *cf = [
                    quad.p0[0], quad.p0[1], quad.p1[0], quad.p1[1], quad.p2[0], quad.p2[1],
                ];
            }
        }
    }
}

#[cfg(debug_assertions)]
fn warn_band_overflow(bands: &[Vec<QuadBez>; BANDS]) {
    for (i, band) in bands.iter().enumerate() {
        if band.len() > MAX_CURVES_PER_BAND {
            eprintln!(
                "sigil-renderer: band {i} overflow ({} curves, max {MAX_CURVES_PER_BAND}); \
                 extra curves dropped at render time",
                band.len()
            );
        }
    }
}
