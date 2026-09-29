#![allow(
    clippy::cast_precision_loss,
    reason = "image dimensions are far below the f32 mantissa limit"
)]

use crate::geometry::{Point, Rect};
use crate::kernel::{ImageCrop, ImageSource, ObjectFit};

use super::payloads::DrawImage;

impl ImageCrop {
    pub(crate) fn rows(self, height: u32) -> Option<(u32, u32)> {
        let top = self.top.min(height);
        let rows = self.height.min(height - top);
        (rows > 0).then_some((top, rows))
    }
}

impl DrawImage {
    pub(crate) fn placement_at(&self, scale: f32) -> Option<(ImageSource, Rect, Rect)> {
        let (source, dest) = self.source_at(scale);
        let (width, height) = (source.width(), source.height());
        let (top, rows) = match self.crop {
            Some(crop) => crop.rows(height)?,
            None => (0, height),
        };
        let region = Rect::new(Point::new(0.0, top as f32), width as f32, rows as f32);
        let (region, destination) = fit(self.fit, region, dest);
        Some((source, region, destination))
    }
}

pub(crate) fn fit(mode: ObjectFit, source: Rect, dest: Rect) -> (Rect, Rect) {
    let (iw, ih) = (source.width, source.height);
    if iw <= 0.0 || ih <= 0.0 || dest.width <= 0.0 || dest.height <= 0.0 {
        return (source, dest);
    }
    match mode {
        ObjectFit::Fill => (source, dest),
        ObjectFit::Contain => {
            let s = (dest.width / iw).min(dest.height / ih);
            let (w, h) = (iw * s, ih * s);
            let origin = Point::new(
                dest.origin.x + (dest.width - w) / 2.0,
                dest.origin.y + (dest.height - h) / 2.0,
            );
            (source, Rect::new(origin, w, h))
        }
        ObjectFit::Cover => {
            let s = (dest.width / iw).max(dest.height / ih);
            let (vw, vh) = (dest.width / s, dest.height / s);
            let origin = Point::new(
                source.origin.x + (iw - vw) / 2.0,
                source.origin.y + (ih - vh) / 2.0,
            );
            (Rect::new(origin, vw, vh), dest)
        }
    }
}
