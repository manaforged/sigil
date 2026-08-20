//! Size + edge resolution helpers for the CSS block solver.
//!
//! Splits the dimension/length math out of [`crate::block`] so the
//! solver entry point stays under the 400-line cap. All helpers are
//! pure functions: they take a containing-block axis size in pixels +
//! a style atom, return resolved pixels (or `None` for `auto`).
//!
//! See DR-041 §"P0.2 block layout" for the conformance scope.

#![allow(clippy::redundant_pub_crate)]

use crate::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::style_data::{BoxSizing, Dimension, Size, StyleData};

/// Per-axis style triple (preferred, min, max) packaged for resolution.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AxisTriple {
    pub(crate) size: Dimension,
    pub(crate) min: Dimension,
    pub(crate) max: Dimension,
}

/// Resolve a preferred + min + max triple on one axis into border-box
/// pixels. Each slot is `None` for `auto`.
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

/// Style-driven axis sizing fed to the block solver per child.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AxisIntent {
    pub(crate) preferred: Option<f32>,
    pub(crate) min: Option<f32>,
    pub(crate) max: Option<f32>,
}

impl AxisIntent {
    /// Build inline-axis intent (width).
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

    /// Build block-axis intent (height).
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

    /// Apply min/max clamp to a candidate border-box pixel value.
    pub(crate) fn clamp(&self, value: f32) -> f32 {
        clamp(value, self.min, self.max)
    }
}

/// Convenience: full size intent on both axes.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SizeIntent {
    pub(crate) w: AxisIntent,
    pub(crate) h: AxisIntent,
}

impl SizeIntent {
    /// Resolve every size axis for `style` against a containing block
    /// of size `cb` and resolved `edges`.
    pub(crate) fn build(style: &StyleData, cb: Size<f32>, edges: &NodeEdges) -> Self {
        Self {
            w: AxisIntent::inline(style, cb.width, edges.inline_chrome()),
            h: AxisIntent::block(style, cb.height, edges.block_chrome()),
        }
    }
}
