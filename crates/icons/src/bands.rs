//! Slug-style 8-band vertical partition of quadratic curves.
//!
//! Each icon's viewBox height is divided into `NUM_BANDS` equal horizontal
//! strips. Every [`QuadCurve`] is assigned to every band whose y-range
//! overlaps the curve's bounding box. The rasterizer then only tests the
//! curves in each pixel's band, keeping per-pixel work small.

use crate::parser::ViewBox;
use crate::quadratic::QuadCurve;

/// Number of vertical bands per icon.
pub(crate) const NUM_BANDS: usize = 8;

/// `NUM_BANDS` as `f32` — avoids a `usize→f32` precision-cast lint at every use.
// NUM_BANDS = 8, well within f32 mantissa range; cast is exact.
#[allow(clippy::cast_precision_loss)]
const NUM_BANDS_F32: f32 = NUM_BANDS as f32;

/// All quadratic curves bucketed into vertical bands.
pub(crate) struct BandBuffer {
    /// `bands[i]` holds the curves whose bounding box overlaps band `i`.
    pub(crate) bands: [Vec<QuadCurve>; NUM_BANDS],
    /// Height of each band in viewBox units.
    pub(crate) band_height: f32,
}

/// Partition `quads` into `NUM_BANDS` vertical bands based on `view_box`.
pub(crate) fn build(quads: &[QuadCurve], view_box: ViewBox) -> BandBuffer {
    let band_height = view_box.height / NUM_BANDS_F32;
    let mut bands: [Vec<QuadCurve>; NUM_BANDS] = core::array::from_fn(|_| Vec::new());

    for &q in quads {
        let y_min = q.start.1.min(q.control.1).min(q.end.1);
        let y_max = q.start.1.max(q.control.1).max(q.end.1);
        let first = band_index(y_min - view_box.y, band_height);
        let last = band_index(y_max - view_box.y, band_height);
        if let Some(slice) = bands.get_mut(first..=last) {
            for band in slice {
                band.push(q);
            }
        }
    }

    BandBuffer { bands, band_height }
}

/// Map a y offset (relative to viewBox origin) to a band index in `[0, NUM_BANDS)`.
fn band_index(y_offset: f32, band_height: f32) -> usize {
    if band_height <= 0.0 {
        return 0;
    }
    let offset = y_offset.max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = (offset / band_height) as usize;
    idx.min(NUM_BANDS - 1)
}
