use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::kernel::{ImageRaster, ImageSource, ThemedImage, cached_image};
use crate::style::Rgba8;

const MAX_SIDE: usize = 4096;

pub(super) struct Grid {
    width: u32,
    height: u32,
    rgba: Rc<[u8]>,
    key: u64,
}

impl Grid {
    pub(super) fn new<F: Fn(usize, usize) -> Rgba8>(
        grid_w: usize,
        grid_h: usize,
        color: &F,
    ) -> Option<Self> {
        if grid_w == 0 || grid_h == 0 || grid_w > MAX_SIDE || grid_h > MAX_SIDE {
            return None;
        }
        let mut rgba = Vec::with_capacity(grid_w * grid_h * 4);
        for y in 0..grid_h {
            for x in 0..grid_w {
                let cell = color(x, y);
                rgba.extend_from_slice(&[cell.r, cell.g, cell.b, cell.a]);
            }
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (grid_w, grid_h, &rgba).hash(&mut hasher);
        Some(Self {
            width: u32::try_from(grid_w).ok()?,
            height: u32::try_from(grid_h).ok()?,
            rgba: Rc::from(rgba),
            key: hasher.finish(),
        })
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "grid sides are capped at MAX_SIDE"
    )]
    pub(super) const fn min_size(&self) -> (f32, f32) {
        (self.width as f32, self.height as f32)
    }

    pub(super) fn source(&self) -> Option<ImageSource> {
        ImageSource::from_rgba8(self.width, self.height, self.rgba.to_vec())
    }

    pub(super) fn themed(&self, source: ImageSource) -> ThemedImage {
        let grid = Rc::new(Self {
            width: self.width,
            height: self.height,
            rgba: Rc::clone(&self.rgba),
            key: self.key,
        });
        ThemedImage::new(
            move |_| Some(source.clone()),
            move |_| {
                let grid = Rc::clone(&grid);
                ImageRaster::new(grid.key, move |width, height, _| grid.scaled(width, height))
            },
        )
    }

    fn scaled(&self, width: u32, height: u32) -> Option<ImageSource> {
        if width == 0 || height == 0 {
            return None;
        }
        let mut key = self.key;
        key ^= u64::from(width).rotate_left(17) ^ u64::from(height).rotate_left(41);
        cached_image(key, || {
            let area = usize::try_from(u64::from(width) * u64::from(height) * 4).ok()?;
            let mut rgba = Vec::with_capacity(area);
            for y in 0..height {
                let row = u64::from(y) * u64::from(self.height) / u64::from(height);
                for x in 0..width {
                    let column = u64::from(x) * u64::from(self.width) / u64::from(width);
                    let at = usize::try_from((row * u64::from(self.width) + column) * 4).ok()?;
                    rgba.extend_from_slice(self.rgba.get(at..at + 4)?);
                }
            }
            ImageSource::from_rgba8(width, height, rgba)
        })
    }
}
