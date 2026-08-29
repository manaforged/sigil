#![allow(clippy::redundant_pub_crate)]

use crate::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::style_data::{BoxSizing, Dimension, Size, StyleData};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AxisTriple {
    pub(crate) size: Dimension,
    pub(crate) min: Dimension,
    pub(crate) max: Dimension,
}

pub(crate) fn axis_sizes(
    triple: AxisTriple,
    cb: f32,
    edges_sum: f32,
    sizing: BoxSizing,
) -> (Option<f32>, Option<f32>, Option<f32>) {
    (
        dim_to_border_box(triple.size, cb, edges_sum, sizing),
        dim_to_border_box(triple.min, cb, edges_sum, sizing),
        dim_to_border_box(triple.max, cb, edges_sum, sizing),
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AxisIntent {
    pub(crate) preferred: Option<f32>,
    pub(crate) min: Option<f32>,
    pub(crate) max: Option<f32>,
}

impl AxisIntent {
    pub(crate) fn inline(style: &StyleData, cb: f32, chrome: f32) -> Self {
        let triple = AxisTriple {
            size: style.size.width,
            min: style.min_size.width,
            max: style.max_size.width,
        };
        let (p, mn, mx) = axis_sizes(triple, cb, chrome, style.box_sizing);
        Self {
            preferred: p,
            min: mn,
            max: mx,
        }
    }

    pub(crate) fn block(style: &StyleData, cb: f32, chrome: f32) -> Self {
        let triple = AxisTriple {
            size: style.size.height,
            min: style.min_size.height,
            max: style.max_size.height,
        };
        let (p, mn, mx) = axis_sizes(triple, cb, chrome, style.box_sizing);
        Self {
            preferred: p,
            min: mn,
            max: mx,
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
            w: AxisIntent::inline(style, cb.width, edges.inline_chrome()),
            h: AxisIntent::block(style, cb.height, edges.block_chrome()),
        }
    }
}
