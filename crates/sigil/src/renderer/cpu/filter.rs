#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "pixel offsets are bounded by the layer size"
)]

use tiny_skia::Pixmap;

use crate::renderer::filter::LayerFilters;
use crate::renderer::filter::matrix::{ColorMatrix, apply, color_matrix};
use crate::scene::command::{Filter, LayerEffect};
use crate::scene::shadow::Shadow;

use super::shadow::blur;
use super::shapes::{decode, encode, encode_byte};

pub(super) fn filter(layer: &mut Pixmap, effect: &LayerEffect, scale: f32) {
    let Some(filters) = LayerFilters::of(effect) else {
        return;
    };
    for step in filters.steps() {
        match *step {
            [Filter::Blur(sigma)] => blur(layer, sigma * scale),
            [Filter::DropShadow(shadow)] => drop_shadow(layer, shadow, scale),
            ref colors => {
                let matrices: Vec<ColorMatrix> =
                    colors.iter().filter_map(|f| color_matrix(*f)).collect();
                recolor(layer, &matrices);
            }
        }
    }
}

fn recolor(layer: &mut Pixmap, matrices: &[ColorMatrix]) {
    let (pixels, _) = layer.data_mut().as_chunks_mut::<4>();
    for pixel in pixels.iter_mut().filter(|pixel| pixel[3] > 0) {
        let alpha = f32::from(pixel[3]) / 255.0;
        let [r, g, b] = [pixel[0], pixel[1], pixel[2]].map(|c| encode(decode(c) / alpha));
        let [r, g, b, a] = matrices
            .iter()
            .fold([r, g, b, alpha], |color, matrix| apply(matrix, color));
        let [r, g, b] = [r, g, b].map(|c| encode_byte(crate::style::srgb::decode_unit(c) * a));
        *pixel = [r, g, b, (a * 255.0).round() as u8];
    }
}

fn drop_shadow(layer: &mut Pixmap, shadow: Shadow, scale: f32) {
    let Some(mut shade) = Pixmap::new(layer.width(), layer.height()) else {
        return;
    };
    shade.data_mut().copy_from_slice(layer.data());
    blur(&mut shade, shadow.blur * scale);
    let [dx, dy] = shadow.offset.map(|value| (value * scale).round() as i64);
    let [r, g, b, a] = shadow.color;
    let tint = [r * a, g * a, b * a, a];
    let (width, height) = (i64::from(layer.width()), i64::from(layer.height()));
    let source = shade.data();
    let (pixels, _) = layer.data_mut().as_chunks_mut::<4>();
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let (x, y) = (index as i64 % width - dx, index as i64 / width - dy);
        let inside = (0..width).contains(&x) && (0..height).contains(&y);
        let coverage = if inside {
            f32::from(source[((y * width + x) * 4 + 3) as usize]) / 255.0
        } else {
            0.0
        };
        *pixel = under(*pixel, tint, coverage);
    }
}

fn under(top: [u8; 4], tint: [f32; 4], coverage: f32) -> [u8; 4] {
    let alpha = f32::from(top[3]) / 255.0;
    let below = 1.0 - alpha;
    let [r, g, b] = [0, 1, 2].map(|i| {
        let shade = tint[i] * coverage * below;
        encode_byte(decode(top[i]) + shade)
    });
    let a = tint[3].mul_add(coverage * below, alpha);
    [r, g, b, (a * 255.0).round().clamp(0.0, 255.0) as u8]
}
