//! 4×4 SSAA CPU rasterizer: band buffer → single-channel alpha mask.
//!
//! Strategy: render at 4× resolution using winding-number coverage per
//! super-sample row, then box-filter 4×4 blocks down to the target size.

use smallvec::SmallVec;

use crate::bands::{BandBuffer, NUM_BANDS};
use crate::parser::ViewBox;
use crate::quadratic::QuadCurve;

/// Super-sample factor per axis.
const SS: u32 = 4;

/// Rasterize `band_buf` into a `width × height` single-channel alpha mask.
///
/// Returns a `Vec<u8>` with `width * height` bytes, row-major, `0..=255`.
pub(super) fn rasterize(
    band_buf: &BandBuffer,
    view_box: ViewBox,
    width: u32,
    height: u32,
) -> Vec<u8> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let ss_w = width * SS;
    let ss_h = height * SS;
    let mut ss_buf = vec![0u8; (ss_w * ss_h) as usize];

    for py in 0..ss_h {
        let vy = ss_to_vb_y(py, ss_h, view_box);
        let band_idx = y_to_band(vy, band_buf, view_box);
        let band = band_buf.bands.get(band_idx).map_or(&[][..], Vec::as_slice);
        for px in 0..ss_w {
            let vx = ss_to_vb_x(px, ss_w, view_box);
            if winding_number(vx, vy, band) != 0
                && let Some(slot) = ss_buf.get_mut((py * ss_w + px) as usize)
            {
                *slot = 255;
            }
        }
    }

    box_filter(&ss_buf, ss_w, width, height)
}

fn ss_to_vb_x(px: u32, ss_w: u32, vb: ViewBox) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let norm = (px as f32 + 0.5) / ss_w as f32;
    vb.x + norm * vb.width
}

fn ss_to_vb_y(py: u32, ss_h: u32, vb: ViewBox) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let norm = (py as f32 + 0.5) / ss_h as f32;
    vb.y + norm * vb.height
}

fn y_to_band(vy: f32, buf: &BandBuffer, view_box: ViewBox) -> usize {
    if buf.band_height <= 0.0 {
        return 0;
    }
    let offset = (vy - view_box.y).max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let idx = (offset / buf.band_height) as usize;
    idx.min(NUM_BANDS - 1)
}

fn winding_number(vx: f32, vy: f32, band: &[QuadCurve]) -> i32 {
    let mut winding = 0_i32;
    for curve in band {
        for param in quad_y_roots(vy, curve) {
            if quad_eval_x(param, curve) > vx {
                let dy = quad_dy_dt(param, curve);
                if dy > 0.0 {
                    winding += 1;
                } else if dy < 0.0 {
                    winding -= 1;
                }
            }
        }
    }
    winding
}

/// Find parameter values in `[0, 1]` where `curve`'s y equals `vy`.
///
/// Solves `coeff_a·t² + coeff_b·t + coeff_c = 0` analytically.
fn quad_y_roots(vy: f32, curve: &QuadCurve) -> SmallVec<[f32; 2]> {
    let (y0, y1, y2) = (curve.start.1, curve.control.1, curve.end.1);
    let coeff_a = 2.0_f32.mul_add(-y1, y0) + y2;
    let coeff_b = 2.0 * (y1 - y0);
    let coeff_c = y0 - vy;
    let mut roots = SmallVec::<[f32; 2]>::new();
    if coeff_a.abs() < 1e-6 {
        if coeff_b.abs() > 1e-6 {
            let root = -coeff_c / coeff_b;
            if (0.0..=1.0).contains(&root) {
                roots.push(root);
            }
        }
    } else {
        let discriminant = coeff_b * coeff_b - 4.0 * coeff_a * coeff_c;
        if discriminant >= 0.0 {
            let sq = discriminant.sqrt();
            for &sign in &[-1.0_f32, 1.0] {
                let root = sign.mul_add(sq, -coeff_b) / (2.0 * coeff_a);
                if (0.0..=1.0).contains(&root) {
                    roots.push(root);
                }
            }
        }
    }
    roots
}

fn quad_eval_x(param: f32, curve: &QuadCurve) -> f32 {
    let inv = 1.0 - param;
    (inv * inv).mul_add(
        curve.start.0,
        (2.0 * inv * param).mul_add(curve.control.0, param * param * curve.end.0),
    )
}

fn quad_dy_dt(param: f32, curve: &QuadCurve) -> f32 {
    let (y0, y1, y2) = (curve.start.1, curve.control.1, curve.end.1);
    let curvature = 2.0_f32.mul_add(-y1, y0) + y2;
    2.0 * curvature.mul_add(param, y1 - y0)
}

fn box_filter(src: &[u8], src_w: u32, out_w: u32, out_h: u32) -> Vec<u8> {
    let ss2 = SS * SS;
    let mut out = vec![0u8; (out_w * out_h) as usize];
    for oy in 0..out_h {
        for ox in 0..out_w {
            let mut sum = 0u32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let idx = ((oy * SS + sy) * src_w + ox * SS + sx) as usize;
                    sum += u32::from(src.get(idx).copied().unwrap_or(0));
                }
            }
            let alpha = ((sum + ss2 / 2) / ss2).min(255) as u8;
            if let Some(slot) = out.get_mut((oy * out_w + ox) as usize) {
                *slot = alpha;
            }
        }
    }
    out
}
