//! Size + edge resolution helpers for the CSS block solver.
//!
//! Splits the dimension/length math out of [`crate::block`] so the
//! solver entry point stays under the 400-line cap. All helpers are
//! pure functions: they take a containing-block axis size in pixels +
//! a style atom, return resolved pixels (or `None` for `auto`).
//!
//! See [DR-041](../../../docs/decisions/DR-041-owned-layout-engine.md)
//! §"P0.2 block layout" for the conformance scope.

#![allow(dead_code, clippy::redundant_pub_crate)]

use crate::style_data::{
    BoxSizing, Dimension, LengthPercentage, LengthPercentageAuto, Rect, Size, StyleData,
};

/// Resolved pixel edges (`Rect<f32>`) on the inline + block axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Edges {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Edges {
    /// Total inline-axis sum (left + right).
    pub(crate) const fn inline(self) -> f32 {
        self.left + self.right
    }

    /// Total block-axis sum (top + bottom).
    pub(crate) const fn block(self) -> f32 {
        self.top + self.bottom
    }
}

/// Resolved auto-aware pixel edges. Each axis edge is `Some(px)` when
/// concrete, `None` when `auto`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AutoEdges {
    pub(crate) left: Option<f32>,
    pub(crate) right: Option<f32>,
    pub(crate) top: Option<f32>,
    pub(crate) bottom: Option<f32>,
}

/// Resolve a `Dimension` against a containing-block axis size.
///
/// `Px(v)` returns `Some(v)`. `Percent(p)` returns `Some(p * cb)` —
/// CSS percent margins/padding always resolve against the containing
/// block's *inline* size; the caller passes the right basis.
/// `Auto` returns `None`. `Fill` returns `Some(cb)` — semantically
/// `Percent(1.0)`, but expresses intent directly (see
/// [`crate::LayoutStyle::fill`]).
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

/// Resolve an auto-capable length/percentage. Returns `None` for
/// `auto`; callers pick the auto policy.
pub(crate) fn resolve_lp_auto(lp: LengthPercentageAuto, cb: f32) -> Option<f32> {
    match lp {
        LengthPercentageAuto::Px(v) => Some(v),
        LengthPercentageAuto::Percent(p) => Some(p * cb),
        LengthPercentageAuto::Auto => None,
    }
}

/// Resolve four-sided length/percentage to pixel `Edges`. Vertical
/// edges (top/bottom) also resolve their percentage against the
/// containing-block inline size — that's the CSS spec rule for both
/// padding and margin percent.
pub(crate) fn edges_lp(rect: Rect<LengthPercentage>, cb_inline: f32) -> Edges {
    Edges {
        left: resolve_lp(rect.left, cb_inline),
        right: resolve_lp(rect.right, cb_inline),
        top: resolve_lp(rect.top, cb_inline),
        bottom: resolve_lp(rect.bottom, cb_inline),
    }
}

/// Resolve auto-capable four-sided edges. Each edge is `Some(px)` when
/// concrete, `None` when `auto`.
pub(crate) fn edges_lp_auto(rect: Rect<LengthPercentageAuto>, cb_inline: f32) -> AutoEdges {
    AutoEdges {
        left: resolve_lp_auto(rect.left, cb_inline),
        right: resolve_lp_auto(rect.right, cb_inline),
        top: resolve_lp_auto(rect.top, cb_inline),
        bottom: resolve_lp_auto(rect.bottom, cb_inline),
    }
}

/// Clamp `value` against optional min/max bounds. `None` means
/// unconstrained on that side. Min wins over max when min > max
/// (per CSS — the min is the floor).
pub(crate) fn clamp(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let after_max = max.map_or(value, |m| value.min(m));
    min.map_or(after_max, |m| after_max.max(m))
}

/// Convert a style content-axis size to the border-box pixel value,
/// honouring `box_sizing`. Returns `None` for `auto`.
///
/// `dim` is the style's preferred / min / max length on this axis.
/// `cb` is the containing-block size on the same axis.
/// `edges_sum` is `padding + border` on this axis (already resolved).
pub(crate) fn dim_to_border_box(
    dim: Dimension,
    cb: f32,
    edges_sum: f32,
    sizing: BoxSizing,
) -> Option<f32> {
    let resolved = resolve_dim(dim, cb)?;
    match sizing {
        BoxSizing::ContentBox => Some(resolved + edges_sum),
        BoxSizing::BorderBox => Some(resolved),
    }
}

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

/// Resolved edges for a node: padding + border (both non-auto) and
/// margin (auto-aware).
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

    /// Concrete margin on the top edge (auto resolves to zero in block
    /// flow; absolute layout has its own auto policy).
    pub(crate) fn margin_top(&self) -> f32 {
        self.margin.top.unwrap_or(0.0)
    }

    /// Concrete margin on the bottom edge.
    pub(crate) fn margin_bottom(&self) -> f32 {
        self.margin.bottom.unwrap_or(0.0)
    }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn dim_px_resolves_directly() {
        assert_eq!(resolve_dim(Dimension::Px(120.0), 500.0), Some(120.0));
    }

    #[test]
    fn dim_percent_resolves_against_basis() {
        let v = resolve_dim(Dimension::Percent(0.25), 800.0).unwrap();
        assert!((v - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dim_auto_returns_none() {
        assert!(resolve_dim(Dimension::Auto, 100.0).is_none());
    }

    #[test]
    fn clamp_applies_min_then_max() {
        assert_eq!(clamp(50.0, Some(60.0), Some(100.0)), 60.0);
        assert_eq!(clamp(200.0, Some(60.0), Some(100.0)), 100.0);
        assert_eq!(clamp(80.0, Some(60.0), Some(100.0)), 80.0);
    }

    #[test]
    fn clamp_min_floors_above_max() {
        // CSS: min wins when min > max.
        assert_eq!(clamp(80.0, Some(120.0), Some(100.0)), 120.0);
    }

    #[test]
    fn border_box_skips_chrome_addition() {
        let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::BorderBox);
        assert_eq!(v, Some(100.0));
    }

    #[test]
    fn content_box_adds_chrome() {
        let v = dim_to_border_box(Dimension::Px(100.0), 0.0, 20.0, BoxSizing::ContentBox);
        assert_eq!(v, Some(120.0));
    }
}
