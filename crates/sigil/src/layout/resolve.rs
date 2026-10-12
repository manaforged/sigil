#![allow(clippy::redundant_pub_crate)]

use crate::layout::snap;
use crate::layout::style_data::{
    BoxSizing, Dimension, LengthPercentage, LengthPercentageAuto, Rect, StyleData,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Edges {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Edges {
    pub(crate) fn map(self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            left: f(self.left),
            right: f(self.right),
            top: f(self.top),
            bottom: f(self.bottom),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AutoEdges {
    pub(crate) left: Option<f32>,
    pub(crate) right: Option<f32>,
    pub(crate) top: Option<f32>,
    pub(crate) bottom: Option<f32>,
}

impl AutoEdges {
    pub(crate) fn or_zero(self) -> Edges {
        Edges {
            left: self.left.unwrap_or(0.0),
            right: self.right.unwrap_or(0.0),
            top: self.top.unwrap_or(0.0),
            bottom: self.bottom.unwrap_or(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NodeEdges {
    pub(crate) padding: Edges,
    pub(crate) border: Edges,
    pub(crate) margin: AutoEdges,
}

impl NodeEdges {
    pub(crate) fn build(style: &StyleData, cb_inline: f32) -> Self {
        Self {
            padding: edges_lp(style.padding, cb_inline).map(snap::gap),
            border: snap::rule_edges(
                edges_lp(style.border, cb_inline),
                fixed_px(style.size.height),
            ),
            margin: snap::gap_auto_edges(edges_lp_auto(style.margin, cb_inline)),
        }
    }

    pub(crate) const fn inline_chrome(&self) -> f32 {
        self.padding.left + self.padding.right + self.border.left + self.border.right
    }

    pub(crate) const fn block_chrome(&self) -> f32 {
        self.padding.top + self.padding.bottom + self.border.top + self.border.bottom
    }

    pub(crate) fn margin_top(&self) -> f32 {
        self.margin.top.unwrap_or(0.0)
    }

    pub(crate) fn margin_bottom(&self) -> f32 {
        self.margin.bottom.unwrap_or(0.0)
    }
}

fn fixed_px(dim: Dimension) -> Option<f32> {
    match dim {
        Dimension::Px(value) => Some(value),
        Dimension::Text(length) => Some(length.px()),
        _ => None,
    }
}

pub(crate) fn resolve_dim(dim: Dimension, cb: f32) -> Option<f32> {
    snap::size(match dim {
        Dimension::Px(v) => Some(v),
        Dimension::Percent(p) => Some(p * cb),
        Dimension::Fill => Some(cb),
        Dimension::Auto => None,
        Dimension::Text(length) => Some(length.px()),
    })
}

pub(crate) fn resolve_lp(lp: LengthPercentage, cb: f32) -> f32 {
    match lp {
        LengthPercentage::Px(v) => v,
        LengthPercentage::Percent(p) => p * cb,
        LengthPercentage::Text(length) => length.px(),
    }
}

fn resolve_lp_auto(lp: LengthPercentageAuto, cb: f32) -> Option<f32> {
    match lp {
        LengthPercentageAuto::Px(v) => Some(v),
        LengthPercentageAuto::Percent(p) => Some(p * cb),
        LengthPercentageAuto::Auto => None,
    }
}

fn edges_lp(rect: Rect<LengthPercentage>, cb_inline: f32) -> Edges {
    Edges {
        left: resolve_lp(rect.left, cb_inline),
        right: resolve_lp(rect.right, cb_inline),
        top: resolve_lp(rect.top, cb_inline),
        bottom: resolve_lp(rect.bottom, cb_inline),
    }
}

fn edges_lp_auto(rect: Rect<LengthPercentageAuto>, cb_inline: f32) -> AutoEdges {
    AutoEdges {
        left: resolve_lp_auto(rect.left, cb_inline),
        right: resolve_lp_auto(rect.right, cb_inline),
        top: resolve_lp_auto(rect.top, cb_inline),
        bottom: resolve_lp_auto(rect.bottom, cb_inline),
    }
}

pub(crate) fn clamp(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let after_max = max.map_or(value, |m| value.min(m));
    min.map_or(after_max, |m| after_max.max(m))
}

pub(crate) fn dim_to_border_box(
    dim: Dimension,
    cb: f32,
    chrome: f32,
    sizing: BoxSizing,
) -> Option<f32> {
    let resolved = resolve_dim(dim, cb)?;
    Some(match sizing {
        BoxSizing::ContentBox => resolved + chrome,
        BoxSizing::BorderBox => resolved,
    })
}

#[derive(Clone, Copy)]
pub(crate) struct InsetAxis {
    pub(crate) leading: LengthPercentageAuto,
    pub(crate) trailing: LengthPercentageAuto,
    pub(crate) origin: f32,
    pub(crate) cb_dim: f32,
    pub(crate) box_dim: f32,
    pub(crate) margin_leading: f32,
    pub(crate) margin_trailing: f32,
}

pub(crate) fn resolve_inset_axis(a: InsetAxis) -> f32 {
    match (a.leading, a.trailing) {
        (LengthPercentageAuto::Auto, t) if !matches!(t, LengthPercentageAuto::Auto) => {
            let off = resolve_lp_auto(t, a.cb_dim).unwrap_or(0.0);
            a.origin + a.cb_dim - a.box_dim - off - a.margin_trailing
        }
        (l, _) => a.origin + resolve_lp_auto(l, a.cb_dim).unwrap_or(0.0) + a.margin_leading,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "resolve_tests.rs"]
mod tests;
