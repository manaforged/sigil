use crate::style::Rgba8;

pub(super) fn tint_toward_hue(c: Rgba8, target_hue_degrees: f32) -> Rgba8 {
    let (_, s, v) = rgb_to_hsv(c);
    hsv_to_rgb(target_hue_degrees, s, v, c.a)
}

pub(super) fn rgb_to_hsv(c: Rgba8) -> (f32, f32, f32) {
    let r = f32::from(c.r) / 255.0;
    let g = f32::from(c.g) / 255.0;
    let b = f32::from(c.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta < f32::EPSILON {
        0.0
    } else if (max - r).abs() < f32::EPSILON {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    let saturation = if max < f32::EPSILON { 0.0 } else { delta / max };
    let value = max;
    (hue, saturation, value)
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32, alpha: u8) -> Rgba8 {
    let scaled_hue = hue.rem_euclid(360.0) / 60.0;
    let sextant = sextant_index(scaled_hue);
    let fract = scaled_hue - scaled_hue.floor();
    let low = value * (1.0 - saturation);
    let falling = value * fract.mul_add(-saturation, 1.0);
    let rising = value * (1.0 - fract).mul_add(-saturation, 1.0);
    let (red, green, blue) = match sextant {
        0 => (value, rising, low),
        1 => (falling, value, low),
        2 => (low, value, rising),
        3 => (low, falling, value),
        4 => (rising, low, value),
        _ => (value, low, falling),
    };
    Rgba8 {
        r: to_byte(red),
        g: to_byte(green),
        b: to_byte(blue),
        a: alpha,
    }
}

fn sextant_index(scaled_hue: f32) -> u8 {
    let floor = scaled_hue.floor();
    if floor <= 0.0 {
        0
    } else if floor >= 5.0 {
        5
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Value gated to 0.0..=5.0 above — exact in u8."
        )]
        {
            floor as u8
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Value clamped to 0.0..=255.0 before cast — bounded."
)]
fn to_byte(channel: f32) -> u8 {
    (channel * 255.0).round().clamp(0.0, 255.0) as u8
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Bounded by definition: (30·255 + 59·255 + 11·255)/100 = 255."
)]
pub(super) const fn perceived_brightness(c: Rgba8) -> u8 {
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    ((r * 30 + g * 59 + b * 11) / 100) as u8
}
