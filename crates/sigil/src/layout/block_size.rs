#![allow(clippy::redundant_pub_crate)]

use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{Dimension, Size, StyleData};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AxisIntent {
    pub(crate) preferred: Option<f32>,
    pub(crate) min: Option<f32>,
    pub(crate) max: Option<f32>,
}

impl AxisIntent {
    fn new(
        style: &StyleData,
        axis: fn(Size<Dimension>) -> Dimension,
        cb: f32,
        chrome: f32,
    ) -> Self {
        let resolve = |dim| dim_to_border_box(axis(dim), cb, chrome, style.box_sizing);
        Self {
            preferred: resolve(style.size),
            min: resolve(style.min_size),
            max: resolve(style.max_size),
        }
    }

    pub(crate) fn clamp(&self, value: f32) -> f32 {
        clamp(value, self.min, self.max)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SizeIntent {
    pub(crate) w: AxisIntent,
    pub(crate) h: AxisIntent,
}

impl SizeIntent {
    pub(crate) fn build(style: &StyleData, cb: Size<f32>, edges: &NodeEdges) -> Self {
        Self {
            w: AxisIntent::new(style, |s| s.width, cb.width, edges.inline_chrome()),
            h: AxisIntent::new(style, |s| s.height, cb.height, edges.block_chrome()),
        }
    }
}
