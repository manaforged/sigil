#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the mask window is clamped to the target size before the casts"
)]

use crate::geometry::Rect;
use tiny_skia::{Mask, MaskType};

use crate::renderer::plan::build_tree;
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::compose;
use crate::scene::command::{DrawPath, Mask as SceneMask, MaskMode};
use crate::scene::path_raster;

use super::paint::CpuPainter;

impl CpuPainter<'_> {
    pub(super) fn path_mask(&self, draw: &DrawPath, bounds: Rect) -> Option<Mask> {
        let (width, height) = (self.target.width(), self.target.height());
        let scale = self.scale;
        let edge = |value: f32, origin: f32, limit: u32| {
            value.mul_add(scale, -origin).clamp(0.0, limit as f32)
        };
        let left = edge(bounds.origin.x, self.origin.x, width).floor();
        let top = edge(bounds.origin.y, self.origin.y, height).floor();
        let right = edge(bounds.origin.x + bounds.width, self.origin.x, width).ceil();
        let bottom = edge(bounds.origin.y + bounds.height, self.origin.y, height).ceil();
        let size = (
            u16::try_from((right - left) as u32).ok()?,
            u16::try_from((bottom - top) as u32).ok()?,
        );
        let device = [
            [scale, 0.0, -self.origin.x - left],
            [0.0, scale, -self.origin.y - top],
        ];
        let current = self.state.current_transform();
        let pixels = path_raster::raster(draw, compose(device, current), size, 1.0)?;
        let mut mask = Mask::new(width, height)?;
        let row = usize::from(size.0);
        let stride = width as usize;
        for (line, source) in pixels.chunks_exact(row * 4).enumerate() {
            let start = (top as usize + line) * stride + left as usize;
            let Some(dest) = mask.data_mut().get_mut(start..start + row) else {
                continue;
            };
            for (alpha, pixel) in dest.iter_mut().zip(source.chunks_exact(4)) {
                *alpha = pixel.get(3).copied().unwrap_or(0);
            }
        }
        Some(mask)
    }

    pub(super) fn scene_mask(
        &mut self,
        mask: &SceneMask,
        region: Rect,
        bounds: Rect,
    ) -> Option<Mask> {
        let state = SceneState::new(Some(bounds), self.state.current_transform(), 1.0);
        let nodes = build_tree(&mask.commands);
        let (pixels, at) = self.render_region(region, state, &nodes)?;
        let kind = match mask.mode {
            MaskMode::Alpha => MaskType::Alpha,
            MaskMode::Luminance => MaskType::Luminance,
        };
        let source = Mask::from_pixmap(pixels.as_ref(), kind);
        self.release_pixmap(pixels);
        let mut placed = Mask::new(self.target.width(), self.target.height())?;
        let (width, height) = (placed.width() as usize, placed.height() as usize);
        let row = source.width() as usize;
        let (left, top) = (at.tx.max(0.0) as usize, at.ty.max(0.0) as usize);
        let span = row.min(width.saturating_sub(left));
        for (line, values) in source.data().chunks_exact(row.max(1)).enumerate() {
            let y = top + line;
            if y >= height {
                break;
            }
            let start = y * width + left;
            if let (Some(dest), Some(values)) = (
                placed.data_mut().get_mut(start..start + span),
                values.get(..span),
            ) {
                dest.copy_from_slice(values);
            }
        }
        Some(placed)
    }
}

pub(super) fn multiplied(mut mask: Mask, by: Option<&Mask>) -> Mask {
    if let Some(by) = by {
        for (value, scale) in mask.data_mut().iter_mut().zip(by.data()) {
            *value = ((u16::from(*value) * u16::from(*scale) + 127) / 255) as u8;
        }
    }
    mask
}
