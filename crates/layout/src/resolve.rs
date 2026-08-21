//! Style-to-pixel resolution shared by every solver.
//!
//! One owner for the CSS length vocabulary: percentages resolve against
//! a containing-block basis, `auto` resolves to `None` so each caller
//! picks its own auto policy, and `box_sizing` decides whether a style
//! size already includes padding + border.
//!
//! Percent padding and margin resolve against the containing block's
//! *inline* size on every edge, including top and bottom — that is the
//! CSS rule, and it is why the edge helpers take one `cb_inline` basis
//! rather than a per-axis pair.
//!
//! See DR-041.

#![allow(clippy::redundant_pub_crate)]

use crate::style_data::{
    BoxSizing, Dimension, LengthPercentage, LengthPercentageAuto, Rect, StyleData,
};

/// Resolved pixel edges (no `auto`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Edges {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Edges {}

/// Resolved pixel edges with auto-aware semantics (margins). Each edge
/// is `Some(px)` when concrete, `None` when `auto`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AutoEdges {
    pub(crate) left: Option<f32>,
    pub(crate) right: Option<f32>,
    pub(crate) top: Option<f32>,
    pub(crate) bottom: Option<f32>,
}

impl AutoEdges {
    /// Collapse every `auto` edge to zero.
    pub(crate) fn or_zero(self) -> Edges {
        Edges {
            left: self.left.unwrap_or(0.0),
            right: self.right.unwrap_or(0.0),
            top: self.top.unwrap_or(0.0),
            bottom: self.bottom.unwrap_or(0.0),
        }
    }
}

/// Resolved padding + border (both non-auto) and margin (auto-aware)
/// for one node.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NodeEdges {
    pub(crate) padding: Edges,
    pub(crate) border: Edges,
    pub(crate) margin: AutoEdges,
}

impl NodeEdges {
    /// Build resolved edges from a style atom and a containing-block
    /// inline size.
    pub(crate) fn build(style: &StyleData, cb_inline: f32) -> Self {
        Self {
            padding: edges_lp(style.padding, cb_inline),
            border: edges_lp(style.border, cb_inline),
            margin: edges_lp_auto(style.margin, cb_inline),
        }
    }

    /// Sum of padding + border on the inline axis.
    pub(crate) const fn inline_chrome(&self) -> f32 {
        self.padding.left + self.padding.right + self.border.left + self.border.right
    }

    /// Sum of padding + border on the block axis.
    pub(crate) const fn block_chrome(&self) -> f32 {
        self.padding.top + self.padding.bottom + self.border.top + self.border.bottom
    }

    /// Concrete margin on the top edge (`auto` resolves to zero in
    /// block flow; absolute layout applies its own auto policy).
    pub(crate) fn margin_top(&self) -> f32 {
        self.margin.top.unwrap_or(0.0)
    }

    /// Concrete margin on the bottom edge.
    pub(crate) fn margin_bottom(&self) -> f32 {
        self.margin.bottom.unwrap_or(0.0)
    }
}

/// Resolve a `Dimension` against a containing-block axis size.
///
/// `Fill` is semantically `Percent(1.0)` but states the intent
/// directly (see [`crate::LayoutStyle::fill`]). `Auto` returns `None`.
pub(crate) fn resolve_dim(dim: Dimension, cb: f32) -> Option<f32> {
    match dim {
        Dimension::Px(v) => Some(v),
        Dimension::Percent(p) => Some(p * cb),
        Dimension::Fill => Some(cb),
        Dimension::Auto => None,
    }
}

/// Resolve a non-`auto` length/percentage against a basis.
pub(crate) fn resolve_lp(lp: LengthPercentage, cb: f32) -> f32 {
    match lp {
        LengthPercentage::Px(v) => v,
        LengthPercentage::Percent(p) => p * cb,
    }
}

/// Resolve an auto-capable length/percentage. `auto` returns `None`;
/// the caller picks the auto policy.
pub(crate) fn resolve_lp_auto(lp: LengthPercentageAuto, cb: f32) -> Option<f32> {
    match lp {
        LengthPercentageAuto::Px(v) => Some(v),
        LengthPercentageAuto::Percent(p) => Some(p * cb),
        LengthPercentageAuto::Auto => None,
    }
}

/// Resolve four-sided length/percentage to pixel [`Edges`].
pub(crate) fn edges_lp(rect: Rect<LengthPercentage>, cb_inline: f32) -> Edges {
    Edges {
        left: resolve_lp(rect.left, cb_inline),
        right: resolve_lp(rect.right, cb_inline),
        top: resolve_lp(rect.top, cb_inline),
        bottom: resolve_lp(rect.bottom, cb_inline),
    }
}

/// Resolve auto-capable four-sided edges to [`AutoEdges`].
pub(crate) fn edges_lp_auto(rect: Rect<LengthPercentageAuto>, cb_inline: f32) -> AutoEdges {
    AutoEdges {
        left: resolve_lp_auto(rect.left, cb_inline),
        right: resolve_lp_auto(rect.right, cb_inline),
        top: resolve_lp_auto(rect.top, cb_inline),
        bottom: resolve_lp_auto(rect.bottom, cb_inline),
    }
}

/// Clamp `value` against optional bounds. `None` is unconstrained on
/// that side. Min wins over max when min > max, per CSS: the min is
/// the floor.
pub(crate) fn clamp(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let after_max = max.map_or(value, |m| value.min(m));
    min.map_or(after_max, |m| after_max.max(m))
}

/// Convert a style axis size to its border-box pixel value under
/// `sizing`. `chrome` is the already-resolved `padding + border` on
/// that axis. Returns `None` for `auto`.
pub(crate) fn dim_to_border_box(
    dim: Dimension,
    cb: f32,
    chrome: f32,
    sizing: BoxSizing,
) -> Option<f32> {
    let resolved = resolve_dim(dim, cb)?;
    match sizing {
        BoxSizing::ContentBox => Some(resolved + chrome),
        BoxSizing::BorderBox => Some(resolved),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
#[path = "resolve_tests.rs"]
mod tests;
