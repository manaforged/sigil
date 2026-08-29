use crate::parser::ViewBox;
use crate::quadratic::QuadCurve;

pub(crate) const NUM_BANDS: usize = 8;

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
const NUM_BANDS_F32: f32 = NUM_BANDS as f32;

pub(crate) struct BandBuffer {
    pub(crate) bands: [Vec<QuadCurve>; NUM_BANDS],
    pub(crate) band_height: f32,
}

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

fn band_index(y_offset: f32, band_height: f32) -> usize {
    if band_height <= 0.0 {
        return 0;
    }
    let offset = y_offset.max(0.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let idx = (offset / band_height) as usize;
    idx.min(NUM_BANDS - 1)
}
