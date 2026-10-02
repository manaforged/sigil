use crate::icons::bands::{BandBuffer, NUM_BANDS};
use crate::icons::parser::ViewBox;
use crate::icons::quadratic::QuadCurve;

const SS: u32 = 4;
const CROSSING_CODES: u32 = 0x2E74;

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
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let norm = (px as f32 + 0.5) / ss_w as f32;
    vb.x + norm * vb.width
}

fn ss_to_vb_y(py: u32, ss_h: u32, vb: ViewBox) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let norm = (py as f32 + 0.5) / ss_h as f32;
    vb.y + norm * vb.height
}

fn y_to_band(vy: f32, buf: &BandBuffer, view_box: ViewBox) -> usize {
    if buf.band_height <= 0.0 {
        return 0;
    }
    let offset = (vy - view_box.y).max(0.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let idx = (offset / buf.band_height) as usize;
    idx.min(NUM_BANDS - 1)
}

fn winding_number(vx: f32, vy: f32, band: &[QuadCurve]) -> i32 {
    let mut winding = 0_i32;
    for curve in band {
        let crossing = crossing_code(vy, curve);
        if crossing == 0 {
            continue;
        }
        let (first, second) = quad_y_roots(vy, curve);
        if crossing & 1 != 0 && quad_eval_x(first, curve) > vx {
            winding += 1;
        }
        if crossing > 1 && quad_eval_x(second, curve) > vx {
            winding -= 1;
        }
    }
    winding
}

fn crossing_code(vy: f32, curve: &QuadCurve) -> u32 {
    let above = |y: f32| u32::from(y > vy);
    let shift =
        (above(curve.start.1) << 1) | (above(curve.control.1) << 2) | (above(curve.end.1) << 3);
    (CROSSING_CODES >> shift) & 3
}

fn quad_y_roots(vy: f32, curve: &QuadCurve) -> (f32, f32) {
    let y0 = curve.start.1 - vy;
    let y1 = curve.control.1 - vy;
    let y2 = curve.end.1 - vy;
    let coeff_a = 2.0_f32.mul_add(-y1, y0) + y2;
    let coeff_b = y0 - y1;
    let root = coeff_b.mul_add(coeff_b, -coeff_a * y0).max(0.0).sqrt();
    let far = if coeff_b >= 0.0 {
        coeff_b + root
    } else {
        coeff_b - root
    };
    let near = y0 / if far == 0.0 { 1.0 } else { far };
    let wide = far / if coeff_a == 0.0 { 1.0 } else { coeff_a };
    let (first, second) = if coeff_b >= 0.0 {
        (near, wide)
    } else {
        (wide, near)
    };
    (first.clamp(0.0, 1.0), second.clamp(0.0, 1.0))
}

fn quad_eval_x(param: f32, curve: &QuadCurve) -> f32 {
    let inv = 1.0 - param;
    (inv * inv).mul_add(
        curve.start.0,
        (2.0 * inv * param).mul_add(curve.control.0, param * param * curve.end.0),
    )
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
