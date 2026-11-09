#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "channels are clamped to 0..=255 before the casts"
)]

use tiny_skia::{Pixmap, Rect as TsRect, Transform as TsTransform};

use super::bound::{corners, extent};

pub(super) fn to_srgb(pixels: &mut [u8]) {
    scale(pixels, f32::sqrt);
}

pub(super) fn to_stored(pixels: &mut [u8]) {
    scale(pixels, |coverage| coverage.sqrt().recip());
}

fn scale(pixels: &mut [u8], gain: fn(f32) -> f32) {
    for [red, green, blue, alpha] in pixels.as_chunks_mut::<4>().0 {
        if *alpha == 0 || *alpha == u8::MAX {
            continue;
        }
        let factor = gain(f32::from(*alpha) / 255.0);
        for channel in [red, green, blue] {
            *channel = (f32::from(*channel) * factor).round().min(255.0) as u8;
        }
    }
}

pub(super) fn convert_under(
    target: &mut Pixmap,
    transform: TsTransform,
    local: TsRect,
    convert: fn(&mut [u8]),
) {
    let rect = crate::geometry::Rect::new(
        crate::geometry::Point::new(local.left(), local.top()),
        local.width(),
        local.height(),
    );
    let [x0, y0, x1, y1] = extent(transform, corners(rect));
    let width = target.width() as usize;
    let clamp = |value: f32, limit: usize| (value.max(0.0) as usize).min(limit);
    let (left, right) = (clamp(x0.floor(), width), clamp(x1.ceil(), width));
    let height = target.height() as usize;
    let (top, bottom) = (clamp(y0.floor(), height), clamp(y1.ceil(), height));
    if left >= right {
        return;
    }
    let data = target.data_mut();
    for row in top..bottom {
        let start = (row * width + left) * 4;
        let end = (row * width + right) * 4;
        if let Some(span) = data.get_mut(start..end) {
            convert(span);
        }
    }
}
