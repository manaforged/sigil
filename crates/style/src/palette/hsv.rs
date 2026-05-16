//! HSV / brightness helpers used by [`super::Palette`] derivation.

use crate::Rgba8;

/// Recolour `c` so its hue matches `target_hue_degrees`, preserving the
/// input's saturation and value. The result keeps the accent's mood
/// (saturated stays saturated, washed-out stays washed-out) while
/// settling on a hue the eye reads as the requested role colour.
///
/// Hue is in degrees `[0, 360)`. Saturation and value are in `[0, 1]`.
/// We never bypass the saturation/value of `c`, so a desaturated grey
/// accent yields desaturated greyish role colours rather than fully
/// saturated rainbow.
pub(super) fn tint_toward_hue(c: Rgba8, target_hue_degrees: f32) -> Rgba8 {
    let (_, s, v) = rgb_to_hsv(c);
    hsv_to_rgb(target_hue_degrees, s, v, c.a)
}

/// Convert an [`Rgba8`] to `(hue_degrees, saturation, value)` in
/// `[0, 360) × [0, 1] × [0, 1]`. Alpha is ignored; callers preserve it.
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

/// Convert `(hue_degrees, saturation, value, alpha)` back to [`Rgba8`].
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32, alpha: u8) -> Rgba8 {
    // Split the hue ring into six 60° sextants. Within each sextant a
    // single channel is at `value`, another at `low`, and the third
    // ramps via `falling`/`rising`. Naming follows that intent rather
    // than the textbook `p`/`q`/`t` single-letter convention.
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

/// Pick the 60°-sextant index for a hue scaled into `[0, 6)`. Always
/// returns 0..=5; values outside `[0, 6)` clamp at the endpoints.
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

/// Round a `[0, 1]` channel to a `u8` byte, clamping defensively in
/// case floating-point drift pushes a value past the endpoint.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Value clamped to 0.0..=255.0 before cast — bounded."
)]
fn to_byte(channel: f32) -> u8 {
    (channel * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Rough perceived-brightness on the **gamma-encoded** (sRGB) channel
/// values, using ITU-R BT.601 weights. This is *not* WCAG relative
/// luminance — that requires de-gamma-ing each channel first, which we
/// skip for cost. Adequate for the dark-vs-light surface decision in
/// [`super::Palette::spellforge`]; do not use it for accessibility
/// contrast ratios.
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
