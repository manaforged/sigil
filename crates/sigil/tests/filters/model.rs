#![allow(
    clippy::suboptimal_flops,
    reason = "the model spells out the spec formula"
)]

use sigil::prelude::Color;
use sigil::scene::Filter;

type Rgba = [f32; 4];

pub(crate) fn linear(channel: u8) -> f32 {
    decode(f32::from(channel) / 255.0)
}

fn decode(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn encode(value: f32) -> f32 {
    let v = value.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

pub(crate) fn gaussian_cdf(z: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.231_641_9 * z.abs());
    let poly = t
        * (0.319_381_53
            + t * (-0.356_563_78 + t * (1.781_477_9 + t * (-1.821_255_9 + t * 1.330_274_5))));
    let tail = (-z * z / 2.0).exp() / (2.0 * std::f32::consts::PI).sqrt() * poly;
    if z >= 0.0 { 1.0 - tail } else { tail }
}

fn rows(matrix: [[f32; 3]; 3], [r, g, b, a]: Rgba) -> Rgba {
    let row = |line: usize| matrix[line][0] * r + matrix[line][1] * g + matrix[line][2] * b;
    [row(0), row(1), row(2), a]
}

fn grayscale(amount: f32, c: Rgba) -> Rgba {
    let k = 1.0 - amount.min(1.0);
    rows(
        [
            [
                0.2126 + 0.7874 * k,
                0.7152 - 0.7152 * k,
                0.0722 - 0.0722 * k,
            ],
            [
                0.2126 - 0.2126 * k,
                0.7152 + 0.2848 * k,
                0.0722 - 0.0722 * k,
            ],
            [
                0.2126 - 0.2126 * k,
                0.7152 - 0.7152 * k,
                0.0722 + 0.9278 * k,
            ],
        ],
        c,
    )
}

fn sepia(amount: f32, c: Rgba) -> Rgba {
    let k = 1.0 - amount.min(1.0);
    rows(
        [
            [0.393 + 0.607 * k, 0.769 - 0.769 * k, 0.189 - 0.189 * k],
            [0.349 - 0.349 * k, 0.686 + 0.314 * k, 0.168 - 0.168 * k],
            [0.272 - 0.272 * k, 0.534 - 0.534 * k, 0.131 + 0.869 * k],
        ],
        c,
    )
}

fn saturate(s: f32, c: Rgba) -> Rgba {
    rows(
        [
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ],
        c,
    )
}

fn hue_rotate(degrees: f32, c: Rgba) -> Rgba {
    let (sin, cos) = degrees.to_radians().sin_cos();
    rows(
        [
            [
                0.213 + cos * 0.787 - sin * 0.213,
                0.715 - cos * 0.715 - sin * 0.715,
                0.072 - cos * 0.072 + sin * 0.928,
            ],
            [
                0.213 - cos * 0.213 + sin * 0.143,
                0.715 + cos * 0.285 + sin * 0.140,
                0.072 - cos * 0.072 - sin * 0.283,
            ],
            [
                0.213 - cos * 0.213 - sin * 0.787,
                0.715 - cos * 0.715 + sin * 0.715,
                0.072 + cos * 0.928 + sin * 0.072,
            ],
        ],
        c,
    )
}

fn transfer([r, g, b, a]: Rgba, curve: impl Fn(f32) -> f32) -> Rgba {
    [curve(r), curve(g), curve(b), a]
}

fn step(filter: Filter, c: Rgba) -> Rgba {
    let out = match filter {
        Filter::Brightness(amount) => transfer(c, |v| amount * v),
        Filter::Contrast(amount) => transfer(c, |v| amount * v - 0.5 * amount + 0.5),
        Filter::Invert(amount) => {
            let a = amount.min(1.0);
            transfer(c, |v| a + v * ((1.0 - a) - a))
        }
        Filter::Grayscale(amount) => grayscale(amount, c),
        Filter::Sepia(amount) => sepia(amount, c),
        Filter::Saturate(amount) => saturate(amount, c),
        Filter::HueRotate(degrees) => hue_rotate(degrees, c),
        Filter::Opacity(amount) => [c[0], c[1], c[2], c[3] * amount.min(1.0)],
        _ => c,
    };
    out.map(|v| v.clamp(0.0, 1.0))
}

pub(crate) fn expected_bytes(filters: &[Filter], input: Color) -> [u8; 3] {
    let start = [input.r, input.g, input.b, input.a].map(|v| f32::from(v) / 255.0);
    let [r, g, b, a] = filters.iter().fold(start, |c, filter| step(*filter, c));
    [r, g, b].map(|srgb| (encode(decode(srgb) * a) * 255.0).round() as u8)
}
