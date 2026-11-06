#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the device window is clamped to the target size before the casts"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{IntSize, Pixmap, Transform as TsTransform};

use crate::renderer::scene_command::walk::{compose, transform_rect};
use crate::scene::command::{DrawPath, Transform};
use crate::scene::path_raster;

use super::paint::CpuPainter;
use super::quad::fill_scaled;
use super::shapes::{Brush, opacity_of};

struct Window {
    x: u32,
    y: u32,
    width: u16,
    height: u16,
}

impl CpuPainter<'_> {
    pub(super) fn path(&mut self, draw: &DrawPath) {
        let current = self.state.current_transform();
        let bounds = transform_rect(current, draw.bounds).expand(1.0 / self.scale);
        let Some(visible) = self.state.clip_rect(bounds) else {
            return;
        };
        let Some(window) = self.window(visible) else {
            return;
        };
        let device = self.device_transform(&window);
        let opacity = opacity_of(draw.opacity, self.state.current_opacity());
        let size = (window.width, window.height);
        let Some(pixels) = path_raster::raster(draw, compose(device, current), size, opacity)
        else {
            return;
        };
        let Some(source) = IntSize::from_wh(size.0.into(), size.1.into())
            .and_then(|size| Pixmap::from_vec(pixels, size))
        else {
            return;
        };
        let (src, dest) = window.rects(self.scale, self.origin);
        let transform = TsTransform::from_scale(self.scale, self.scale)
            .post_translate(-self.origin.x, -self.origin.y);
        if self.with_brush(visible, 1.0, true, |target, _, brush| {
            let device = Brush {
                transform,
                ..*brush
            };
            fill_scaled(target, &source, src, dest, &device);
            true
        }) {
            self.stats.sprite_draws += 1;
        }
    }

    fn window(&self, visible: Rect) -> Option<Window> {
        let left = (visible.origin.x * self.scale - self.origin.x).floor().max(0.0);
        let top = (visible.origin.y * self.scale - self.origin.y).floor().max(0.0);
        let right = ((visible.origin.x + visible.width) * self.scale - self.origin.x)
            .ceil()
            .min(self.target.width() as f32);
        let bottom = ((visible.origin.y + visible.height) * self.scale - self.origin.y)
            .ceil()
            .min(self.target.height() as f32);
        let side = |extent: f32| u16::try_from(extent as u32).ok().filter(|side| *side > 0);
        Some(Window {
            x: left as u32,
            y: top as u32,
            width: side(right - left)?,
            height: side(bottom - top)?,
        })
    }

    fn device_transform(&self, window: &Window) -> Transform {
        let shift = |origin: f32, at: u32| -origin - at as f32;
        [
            [self.scale, 0.0, shift(self.origin.x, window.x)],
            [0.0, self.scale, shift(self.origin.y, window.y)],
        ]
    }
}

impl Window {
    fn rects(&self, scale: f32, origin: Point) -> (Rect, Rect) {
        let (width, height) = (f32::from(self.width), f32::from(self.height));
        let src = Rect::new(Point::new(0.0, 0.0), width, height);
        let dest = Rect::new(
            Point::new(
                (self.x as f32 + origin.x) / scale,
                (self.y as f32 + origin.y) / scale,
            ),
            width / scale,
            height / scale,
        );
        (src, dest)
    }
}
