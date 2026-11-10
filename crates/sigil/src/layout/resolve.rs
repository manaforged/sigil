#![allow(clippy::redundant_pub_crate)]

use crate::layout::snap;
use crate::layout::style_data::{
    Axes, BoxSizing, Edges, Length, LengthPercentage, LengthPercentageAuto, StyleData,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ResolvedEdges {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl ResolvedEdges {
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
    pub(crate) fn or_zero(self) -> ResolvedEdges {
        ResolvedEdges {
            left: self.left.unwrap_or(0.0),
            right: self.right.unwrap_or(0.0),
            top: self.top.unwrap_or(0.0),
            bottom: self.bottom.unwrap_or(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NodeEdges {
    pub(crate) padding: ResolvedEdges,
    pub(crate) border: ResolvedEdges,
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

fn fixed_px(dim: Length) -> Option<f32> {
    match dim {
        Length::Px(value) => Some(value),
        Length::Rows(_) | Length::Cols(_) | Length::Em(_) => dim.text_px(),
        _ => None,
    }
}

pub(crate) fn resolve_dim(dim: Length, cb: f32) -> Option<f32> {
    snap::size(match dim {
        Length::Px(v) => Some(v),
        Length::Percent(p) => Some(p * cb),
        Length::Fill => Some(cb),
        Length::Auto => None,
        Length::Rows(_) | Length::Cols(_) | Length::Em(_) => dim.text_px(),
    })
}

pub(crate) fn resolve_lp(lp: Length, cb: f32) -> f32 {
    match lp.definite() {
        Some(LengthPercentage::Px(px)) => px,
        Some(LengthPercentage::Percent(percent)) => percent * cb,
        Some(LengthPercentage::Text(text)) => text.px(),
        None => 0.0,
    }
}

fn resolve_lp_auto(lp: LengthPercentageAuto, cb: f32) -> Option<f32> {
    match lp {
        LengthPercentageAuto::Px(v) => Some(v),
        LengthPercentageAuto::Percent(p) => Some(p * cb),
        LengthPercentageAuto::Auto => None,
    }
}

fn edges_lp(rect: Edges<Length>, cb_inline: f32) -> ResolvedEdges {
    ResolvedEdges {
        left: resolve_lp(rect.left, cb_inline),
        right: resolve_lp(rect.right, cb_inline),
        top: resolve_lp(rect.top, cb_inline),
        bottom: resolve_lp(rect.bottom, cb_inline),
    }
}

fn edges_lp_auto(rect: Edges<Length>, cb_inline: f32) -> AutoEdges {
    AutoEdges {
        left: resolve_lp_auto(rect.left.into(), cb_inline),
        right: resolve_lp_auto(rect.right.into(), cb_inline),
        top: resolve_lp_auto(rect.top.into(), cb_inline),
        bottom: resolve_lp_auto(rect.bottom.into(), cb_inline),
    }
}

pub(crate) fn clamp(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let after_max = max.map_or(value, |m| value.min(m));
    min.map_or(after_max, |m| after_max.max(m))
}

pub(crate) fn dim_to_border_box(
    dim: Length,
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
    pub(crate) leading: Length,
    pub(crate) trailing: Length,
    pub(crate) origin: f32,
    pub(crate) cb_dim: f32,
    pub(crate) box_dim: f32,
    pub(crate) margin_leading: f32,
    pub(crate) margin_trailing: f32,
}

pub(crate) fn resolve_inset_axis(a: InsetAxis) -> f32 {
    match (
        LengthPercentageAuto::from(a.leading),
        LengthPercentageAuto::from(a.trailing),
    ) {
        (LengthPercentageAuto::Auto, t) if !matches!(t, LengthPercentageAuto::Auto) => {
            let off = resolve_lp_auto(t, a.cb_dim).unwrap_or(0.0);
            a.origin + a.cb_dim - a.box_dim - off - a.margin_trailing
        }
        (l, _) => a.origin + resolve_lp_auto(l, a.cb_dim).unwrap_or(0.0) + a.margin_leading,
    }
}

pub(crate) fn stretched(
    [leading, trailing]: [Length; 2],
    cb_dim: f32,
    margins: f32,
) -> Option<f32> {
    let leading = resolve_lp_auto(leading.into(), cb_dim)?;
    let trailing = resolve_lp_auto(trailing.into(), cb_dim)?;
    Some((cb_dim - leading - trailing - margins).max(0.0))
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

pub(crate) fn resolve_absolute_xy(
    style: &StyleData,
    edges: &NodeEdges,
    (origin_x, origin_y): (f32, f32),
    padding_box: Axes<f32>,
    (box_w, box_h): (f32, f32),
) -> (f32, f32) {
    let x = resolve_inset_axis(InsetAxis {
        leading: style.inset.left,
        trailing: style.inset.right,
        origin: origin_x,
        cb_dim: padding_box.width,
        box_dim: box_w,
        margin_leading: edges.margin.left.unwrap_or(0.0),
        margin_trailing: edges.margin.right.unwrap_or(0.0),
    });
    let y = resolve_inset_axis(InsetAxis {
        leading: style.inset.top,
        trailing: style.inset.bottom,
        origin: origin_y,
        cb_dim: padding_box.height,
        box_dim: box_h,
        margin_leading: edges.margin_top(),
        margin_trailing: edges.margin.bottom.unwrap_or(0.0),
    });
    (x, y)
}
