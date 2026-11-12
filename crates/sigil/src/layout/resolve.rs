#![allow(clippy::redundant_pub_crate)]

use crate::layout::snap;
use crate::layout::style_data::{
    Axes, BoxSizing, Display, Edges, Length, LengthPercentage, LengthPercentageAuto, Position,
    StyleData,
};

pub(crate) type ResolvedEdges = Edges<f32>;
pub(crate) type AutoEdges = Edges<Option<f32>>;

impl AutoEdges {
    pub(crate) fn or_zero(self) -> ResolvedEdges {
        self.map(|edge| edge.unwrap_or(0.0))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NodeEdges {
    pub(crate) padding: ResolvedEdges,
    pub(crate) border: ResolvedEdges,
    pub(crate) margin: AutoEdges,
}

fn zero(edges: Edges<Length>) -> bool {
    let zero = |side: Length| matches!(side, Length::Px(px) if px == 0.0);
    zero(edges.left) && zero(edges.right) && zero(edges.top) && zero(edges.bottom)
}

const ZERO_RESOLVED: ResolvedEdges = ResolvedEdges {
    left: 0.0,
    right: 0.0,
    top: 0.0,
    bottom: 0.0,
};
const ZERO_AUTO: AutoEdges = AutoEdges {
    left: Some(0.0),
    right: Some(0.0),
    top: Some(0.0),
    bottom: Some(0.0),
};

impl NodeEdges {
    pub(crate) const BARE: Self = Self {
        padding: ZERO_RESOLVED,
        border: ZERO_RESOLVED,
        margin: ZERO_AUTO,
    };

    pub(crate) fn bare(style: &StyleData) -> bool {
        zero(style.padding) && zero(style.border) && zero(style.margin)
    }

    pub(crate) fn build(style: &StyleData, cb_inline: f32) -> Self {
        let padding = if zero(style.padding) {
            ZERO_RESOLVED
        } else {
            edges_lp(style.padding, cb_inline).map(snap::gap)
        };
        let border = if zero(style.border) {
            ZERO_RESOLVED
        } else {
            snap::rule_edges(
                edges_lp(style.border, cb_inline),
                fixed_px(style.size.height),
            )
        };
        let margin = if zero(style.margin) {
            ZERO_AUTO
        } else {
            snap::gap_auto_edges(edges_lp_auto(style.margin, cb_inline))
        };
        Self {
            padding,
            border,
            margin,
        }
    }

    pub(crate) const fn inline_chrome(&self) -> f32 {
        self.padding.left + self.padding.right + self.border.left + self.border.right
    }

    pub(crate) const fn block_chrome(&self) -> f32 {
        self.padding.top + self.padding.bottom + self.border.top + self.border.bottom
    }

    pub(crate) fn padding_box(&self, (w, h): (f32, f32)) -> Axes<f32> {
        Axes {
            width: (w - self.border.left - self.border.right).max(0.0),
            height: (h - self.border.top - self.border.bottom).max(0.0),
        }
    }

    pub(crate) fn margin_top(&self) -> f32 {
        self.margin.top.unwrap_or(0.0)
    }

    pub(crate) fn margin_bottom(&self) -> f32 {
        self.margin.bottom.unwrap_or(0.0)
    }
}

pub(crate) const fn is_positioned_absolute(style: &StyleData) -> bool {
    matches!(style.position, Position::Absolute) && !matches!(style.display, Display::None)
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
    rect.map(|edge| resolve_lp(edge, cb_inline))
}

fn edges_lp_auto(rect: Edges<Length>, cb_inline: f32) -> AutoEdges {
    rect.map(|edge| resolve_lp_auto(edge.into(), cb_inline))
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
