use crate::hal::{Point, Rect};
use crate::scene::corner_radii::CornerRadii;

use super::stroke_band::StrokeBand;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Trimmed {
    pub(crate) rect: Rect,
    pub(crate) radii: CornerRadii,
    pub(crate) shift: [f64; 2],
}

impl Trimmed {
    pub(crate) const fn whole(rect: Rect, radii: CornerRadii) -> Self {
        Self {
            rect,
            radii,
            shift: [0.0, 0.0],
        }
    }

    pub(crate) fn dash_origin(&self, original: Rect, band: Option<StrokeBand>) -> [f32; 2] {
        let Some((dash, band)) = band.and_then(|b| b.dash.map(|d| (d, b))) else {
            return [0.0, 0.0];
        };
        let reach = band.outside + DASH_REACH_PAD;
        let [dx, dy] = self.shift;
        let axis = |shift: f64, kept: f32, whole: f32| {
            let offset = (f64::from(kept) - f64::from(whole)).mul_add(0.5, shift);
            dash.fold(offset, kept.mul_add(0.5, reach))
        };
        [
            axis(dx, self.rect.width, original.width),
            axis(dy, self.rect.height, original.height),
        ]
    }
}

const DASH_REACH_PAD: f32 = 4.0;

pub(crate) fn trim_to_view(
    rect: Rect,
    radii: CornerRadii,
    view: Rect,
    margin: f32,
) -> Option<Trimmed> {
    let base = radii.normalized_for(rect.width, rect.height);
    let reach = margin + base.iter().copied().fold(0.0, f32::max);
    let (x0, x1, left, right) = cut(
        rect.origin.x,
        rect.origin.x + rect.width,
        view.origin.x - reach,
        view.origin.x + view.width + reach,
    )?;
    let (y0, y1, top, bottom) = cut(
        rect.origin.y,
        rect.origin.y + rect.height,
        view.origin.y - reach,
        view.origin.y + view.height + reach,
    )?;
    let [tl, tr, br, bl] = base;
    let keep = |r: f32, gone: bool| if gone { 0.0 } else { r };
    Some(Trimmed {
        rect: Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0),
        radii: CornerRadii::new(
            keep(tl, left || top),
            keep(tr, right || top),
            keep(br, right || bottom),
            keep(bl, left || bottom),
        ),
        shift: [
            f64::from(x0) - f64::from(rect.origin.x),
            f64::from(y0) - f64::from(rect.origin.y),
        ],
    })
}

fn cut(lo: f32, hi: f32, view_lo: f32, view_hi: f32) -> Option<(f32, f32, bool, bool)> {
    let low_cut = lo < view_lo;
    let high_cut = hi > view_hi;
    let a = if low_cut { view_lo } else { lo };
    let b = if high_cut { view_hi } else { hi };
    (b >= a).then_some((a, b, low_cut, high_cut))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the phase is reduced below one tile before narrowing"
)]
pub(crate) fn tile_phase(shift: [f64; 2], tile: f32) -> [f32; 2] {
    let tile = f64::from(tile);
    if !(tile.is_finite() && tile > 0.0) {
        return [0.0, 0.0];
    }
    shift.map(|s| {
        let phase = s.rem_euclid(tile);
        if phase.is_finite() { phase as f32 } else { 0.0 }
    })
}
