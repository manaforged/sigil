//! Glyph outline → horizontal band partition (CPU preprocessing).
//!
//! For each glyph we build a flat `f32` buffer of 16 horizontal bands,
//! each holding up to 16 normalised quadratic Bézier arcs. The GPU shader
//! indexes into this buffer by fragment UV to compute winding coverage.

use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, pen::PathElement},
};

use super::curve::{QuadBez, cubic_to_quads, line_to_quad};

/// Number of horizontal bands per glyph.
pub(super) const BANDS: usize = 16;
/// Maximum quadratic arcs stored per band.
///
/// A curve is bucketed into every band its extent spans, so a glyph's middle
/// band accumulates every stroke that crosses the x-height — dense Latin glyphs
/// (`@`, `&`, `B`, nested brackets) and most CJK reach into the high teens. At 16
/// those overflowed and the surplus curves were silently dropped at pack time,
/// punching holes in the affected glyph. 32 clears the observed Latin worst case
/// (~19) with headroom; the GPU stride below derives from this, so the shader's
/// `MAX_CURVES` / `FLOATS_PER_BAND` constants must be kept in lockstep.
pub(super) const MAX_CURVES_PER_BAND: usize = 32;
/// `f32` entries per band: 1 (count) + `MAX_CURVES_PER_BAND` × 6 (p0, p1, p2).
pub(super) const FLOATS_PER_BAND: usize = 1 + MAX_CURVES_PER_BAND * 6;
/// `f32` entries for one axis's `BANDS` bands (the horizontal OR the vertical set).
pub(super) const FLOATS_PER_AXIS: usize = BANDS * FLOATS_PER_BAND;
/// Total `f32` entries in one `BandData` buffer: horizontal bands (partitioned by
/// y, for the +x coverage ray) followed by vertical bands (partitioned by x, for
/// the +y ray). Two-axis coverage is what anti-aliases near-horizontal edges.
pub(super) const FLOATS_TOTAL: usize = 2 * FLOATS_PER_AXIS;

/// Glyph bounding box in design (unscaled) coordinates.
#[derive(Copy, Clone, Debug)]
pub(super) struct GlyphBbox {
    /// Minimum x (left edge).
    pub x_min: f32,
    /// Minimum y (bottom in y-up design space, i.e., descender).
    pub y_min: f32,
    /// Maximum x (right edge).
    pub x_max: f32,
    /// Maximum y (top in y-up design space, i.e., ascender).
    pub y_max: f32,
}

impl GlyphBbox {
    /// Bbox width in design units.
    pub(super) fn width(self) -> f32 {
        self.x_max - self.x_min
    }
    /// Bbox height in design units.
    pub(super) fn height(self) -> f32 {
        self.y_max - self.y_min
    }
}

/// Preprocessed glyph data: band-partitioned quadratic arcs in normalised \[0,1\] coords.
///
/// `y = 0` maps to the top of the glyph (ascender), `y = 1` to the bottom (descender).
/// Coordinates are normalised relative to the glyph bounding box so the data is
/// resolution-independent — one `BandData` entry serves all font sizes.
#[derive(Clone)]
pub(super) struct BandData {
    /// Flat float buffer of `BANDS × FLOATS_PER_BAND` entries.
    pub data: Vec<f32>,
    /// Glyph bbox in unscaled design units (for pixel-quad sizing at render time).
    pub bbox: GlyphBbox,
    /// Font units-per-em; used to scale design units to pixels at render time.
    pub units_per_em: u16,
}

impl BandData {
    /// Raw bytes of the band data buffer, ready for GPU upload.
    pub(super) fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.data)
    }
}

/// Preprocess a glyph into band-partitioned quadratic arcs.
///
/// `variation_coords` are raw `i16` fixed-point normalised-variation bits
/// as produced by the owned text engine via skrifa's variation tables.
/// Pass an empty slice for static (non-variable) fonts.
///
/// Returns `None` if the glyph has no outline (spaces, control characters, etc.).
pub(super) fn preprocess_glyph(
    font_bytes: &[u8],
    font_index: u32,
    glyph_id: u32,
    variation_coords: &[i16],
) -> Option<BandData> {
    let (elements, upem) = collect_outline(font_bytes, font_index, glyph_id, variation_coords)?;
    let quads = path_elements_to_quads(&elements);
    if quads.is_empty() {
        return None;
    }
    let bbox = compute_bbox(&quads)?;
    if bbox.width() < 1e-4 || bbox.height() < 1e-4 {
        return None;
    }
    let normalised: Vec<QuadBez> = quads.iter().map(|q| normalise_quad(q, bbox)).collect();
    Some(pack_into_bands(&normalised, bbox, upem))
}

/// Load the font, extract the glyph outline, and return path elements + upem.
fn collect_outline(
    font_bytes: &[u8],
    font_index: u32,
    glyph_id: u32,
    variation_coords: &[i16],
) -> Option<(Vec<PathElement>, u16)> {
    let font = FontRef::from_index(font_bytes, font_index).ok()?;
    let upem = font
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em;
    let glyph = font.outline_glyphs().get(GlyphId::new(glyph_id))?;
    let coords: Vec<NormalizedCoord> = variation_coords
        .iter()
        .map(|&c| NormalizedCoord::from_bits(c))
        .collect();
    let location = LocationRef::new(&coords);
    let settings = DrawSettings::unhinted(Size::unscaled(), location);
    let mut elements: Vec<PathElement> = Vec::new();
    glyph.draw(settings, &mut elements).ok()?;
    if elements.is_empty() {
        return None;
    }
    Some((elements, upem))
}

/// Convert skrifa path elements to a flat list of quadratic Bézier arcs.
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

/// Compute the axis-aligned bounding box of a set of quadratic arcs.
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
    if x_min > x_max {
        None
    } else {
        Some(GlyphBbox {
            x_min,
            y_min,
            x_max,
            y_max,
        })
    }
}

/// Normalise a quad's coordinates to \[0, 1\] within `bbox`.
///
/// Y is flipped so that `y = 0` is the top of the bbox (highest design y)
/// and `y = 1` is the bottom — matching screen / UV convention.
fn normalise_quad(quad: &QuadBez, bbox: GlyphBbox) -> QuadBez {
    let w = bbox.width();
    let h = bbox.height();
    let n =
        |pt: [f32; 2]| -> [f32; 2] { [(pt[0] - bbox.x_min) / w, 1.0 - (pt[1] - bbox.y_min) / h] };
    QuadBez {
        p0: n(quad.p0),
        p1: n(quad.p1),
        p2: n(quad.p2),
    }
}

/// Partition normalised quads into two band sets and pack into a flat `f32`
/// buffer: horizontal bands (by y-extent) then vertical bands (by x-extent). The
/// shader casts a +x ray against the horizontal set and a +y ray against the
/// vertical set, so near-horizontal edges get antialiased instead of jagged.
fn pack_into_bands(quads: &[QuadBez], bbox: GlyphBbox, units_per_em: u16) -> BandData {
    let h_bands = partition_bands(quads, |q| {
        (
            q.p0[1].min(q.p1[1]).min(q.p2[1]),
            q.p0[1].max(q.p1[1]).max(q.p2[1]),
        )
    });
    let v_bands = partition_bands(quads, |q| {
        (
            q.p0[0].min(q.p1[0]).min(q.p2[0]),
            q.p0[0].max(q.p1[0]).max(q.p2[0]),
        )
    });

    #[cfg(debug_assertions)]
    {
        warn_band_overflow(&h_bands);
        warn_band_overflow(&v_bands);
    }

    let mut data = vec![0.0f32; FLOATS_TOTAL];
    let (h_out, v_out) = data.split_at_mut(FLOATS_PER_AXIS);
    pack_bands(&h_bands, h_out);
    pack_bands(&v_bands, v_out);

    BandData {
        data,
        bbox,
        units_per_em,
    }
}

/// Bucket each quad into the `BANDS` bands its `extent` (normalised lo, hi) spans.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn partition_bands(
    quads: &[QuadBez],
    extent: impl Fn(&QuadBez) -> (f32, f32),
) -> [Vec<QuadBez>; BANDS] {
    let mut bands: [Vec<QuadBez>; BANDS] = core::array::from_fn(|_| Vec::new());
    for quad in quads {
        let (lo, hi) = extent(quad);
        let b_lo = ((lo * BANDS as f32).floor() as usize).min(BANDS - 1);
        let b_hi = ((hi * BANDS as f32).ceil() as usize).min(BANDS);
        for b in b_lo..b_hi {
            if let Some(band) = bands.get_mut(b) {
                band.push(*quad);
            }
        }
    }
    bands
}

/// Pack one axis's bands into `out` (exactly `FLOATS_PER_AXIS` entries): per band,
/// a clamped curve count followed by up to `MAX_CURVES_PER_BAND` × 6 coordinates.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn pack_bands(band_curves: &[Vec<QuadBez>; BANDS], out: &mut [f32]) {
    for (chunk, band) in out
        .chunks_exact_mut(FLOATS_PER_BAND)
        .zip(band_curves.iter())
    {
        if let Some((count_slot, curves_data)) = chunk.split_first_mut() {
            // Clamp the count — the GPU buffer holds exactly MAX_CURVES_PER_BAND
            // slots; extra curves are not packed but we collected them to detect
            // overflow in debug builds.
            *count_slot = band.len().min(MAX_CURVES_PER_BAND) as f32;
            for (cf, quad) in curves_data.chunks_exact_mut(6).zip(band.iter()) {
                if let [p0x, p0y, p1x, p1y, p2x, p2y] = cf {
                    *p0x = quad.p0[0];
                    *p0y = quad.p0[1];
                    *p1x = quad.p1[0];
                    *p1y = quad.p1[1];
                    *p2x = quad.p2[0];
                    *p2y = quad.p2[1];
                }
            }
        }
    }
}

/// Emit a debug warning for any band that exceeds `MAX_CURVES_PER_BAND`.
///
/// Compiled out in release builds. Call once per glyph after the collection
/// loop so callers know when complex glyphs are silently truncated.
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
