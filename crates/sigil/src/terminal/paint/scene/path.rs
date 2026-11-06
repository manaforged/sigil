#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the raster side is clamped to the u16 range before the cast"
)]

use super::*;
use crate::kernel::{ImageSource, cached_image};
use crate::scene::DrawPath;
use crate::scene::path_raster;

const RASTER_SCALE: f32 = 2.0;
const MAX_SIDE: f32 = 4096.0;

impl CellWalk<'_> {
    pub(super) fn path(&mut self, draw: &DrawPath) {
        let Some(source) = path_image(draw) else {
            return;
        };
        self.image(&DrawImage {
            node: draw.node,
            source,
            fit: ObjectFit::Fill,
            destination: draw.bounds,
            opacity: draw.opacity,
            crop: None,
            raster: None,
        });
    }
}

fn path_image(draw: &DrawPath) -> Option<ImageSource> {
    let side = |extent: f32| (extent * RASTER_SCALE).ceil().clamp(1.0, MAX_SIDE) as u16;
    let size = (side(draw.bounds.width), side(draw.bounds.height));
    let origin = draw.bounds.origin;
    let device = [
        [RASTER_SCALE, 0.0, -origin.x * RASTER_SCALE],
        [0.0, RASTER_SCALE, -origin.y * RASTER_SCALE],
    ];
    cached_image(draw.key_at(RASTER_SCALE), || {
        let mut pixels = path_raster::raster(draw, device, size, 1.0)?;
        unpremultiply(&mut pixels);
        ImageSource::from_rgba8(u32::from(size.0), u32::from(size.1), pixels)
    })
}

fn unpremultiply(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}
