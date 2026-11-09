#![allow(
    clippy::suboptimal_flops,
    reason = "the model spells out the spec formula"
)]

use sigil::scene::BlendMode;

type Rgb = [f64; 3];

fn separable(mode: BlendMode, cb: f64, cs: f64) -> f64 {
    match mode {
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => screen(cb, cs),
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::Darken => cb.min(cs),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::ColorDodge if cb == 0.0 => 0.0,
        BlendMode::ColorDodge if cs >= 1.0 => 1.0,
        BlendMode::ColorDodge => (cb / (1.0 - cs)).min(1.0),
        BlendMode::ColorBurn if cb >= 1.0 => 1.0,
        BlendMode::ColorBurn if cs == 0.0 => 0.0,
        BlendMode::ColorBurn => 1.0 - ((1.0 - cb) / cs).min(1.0),
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::SoftLight => soft_light(cb, cs),
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        _ => cs,
    }
}

fn screen(cb: f64, cs: f64) -> f64 {
    cb + cs - cb * cs
}

fn hard_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        screen(cb, 2.0 * cs - 1.0)
    }
}

fn soft_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
    }
    let d = if cb <= 0.25 {
        ((16.0 * cb - 12.0) * cb + 4.0) * cb
    } else {
        cb.sqrt()
    };
    cb + (2.0 * cs - 1.0) * (d - cb)
}

fn lum([r, g, b]: Rgb) -> f64 {
    0.3 * r + 0.59 * g + 0.11 * b
}

fn clip_color(c: Rgb) -> Rgb {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        out = out.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        out = out.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    out
}

fn set_lum(c: Rgb, l: f64) -> Rgb {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn sat(c: Rgb) -> f64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: Rgb, s: f64) -> Rgb {
    let mut order = [0, 1, 2];
    order.sort_by(|a, b| c[*a].total_cmp(&c[*b]));
    let [min, mid, max] = order;
    let mut out = [0.0; 3];
    if c[max] > c[min] {
        out[mid] = (c[mid] - c[min]) * s / (c[max] - c[min]);
        out[max] = s;
    }
    out
}

fn blend(mode: BlendMode, cb: Rgb, cs: Rgb) -> Rgb {
    match mode {
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
        _ => [0, 1, 2].map(|i| separable(mode, cb[i], cs[i])),
    }
}

pub(super) fn composite(mode: BlendMode, backdrop: [u8; 3], source: [u8; 3], alpha: u8) -> [u8; 3] {
    let unit = |c: [u8; 3]| c.map(|v| f64::from(v) / 255.0);
    let (cb, cs, a) = (unit(backdrop), unit(source), f64::from(alpha) / 255.0);
    let mixed = blend(mode, cb, cs);
    [0, 1, 2].map(|i| ((a * mixed[i] + (1.0 - a) * cb[i]) * 255.0).round() as u8)
}
