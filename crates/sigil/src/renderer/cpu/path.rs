#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the device window is clamped to the target size before the casts"
)]

use crate::geometry::Rect;
use tiny_skia::{BlendMode, IntSize, Pixmap, Transform as TsTransform};

use crate::renderer::scene_command::walk::{compose, transform_rect};
use crate::scene::command::{DrawPath, Transform};
use crate::scene::path_raster;

use super::encoding::to_stored;
use super::paint::CpuPainter;
use super::shapes::{BLEND_SPACE, blit_in, opacity_of};

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
        let Some(mut pixels) = path_raster::raster(draw, compose(device, current), size, opacity)
        else {
            return;
        };
        to_stored(&mut pixels);
        let Some(source) = IntSize::from_wh(size.0.into(), size.1.into())
            .and_then(|size| Pixmap::from_vec(pixels, size))
        else {
            return;
        };
        let at = TsTransform::from_translate(window.x as f32, window.y as f32);
        let ink = (BlendMode::SourceOver, BLEND_SPACE, 1.0);
        if self.with_brush(visible, 1.0, true, |target, _, brush| {
            blit_in(target, &source, at, brush.coverage(), ink);
            true
        }) {
            self.stats.sprite_draws += 1;
        }
    }

    fn window(&self, visible: Rect) -> Option<Window> {
        let left = (visible.origin.x * self.scale - self.origin.x)
            .floor()
            .max(0.0);
        let top = (visible.origin.y * self.scale - self.origin.y)
            .floor()
            .max(0.0);
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
