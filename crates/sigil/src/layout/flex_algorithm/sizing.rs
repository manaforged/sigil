use crate::layout::forest::NodeRef;
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{AlignSelf, Dimension, Overflow, StyleData};

use super::intrinsic::{container_intrinsic_cross, container_intrinsic_main, text_intrinsic};
use super::{Axis2, FlexCtx};

pub(super) fn container_inner_size(
    ctx: &FlexCtx<'_, '_>,
    style: &StyleData,
    edges: &NodeEdges,
) -> (f32, f32) {
    let (cb_main, cb_cross) = ctx.cb.along(ctx.axes);
    let (size_main, size_cross) = main_cross_size(style, ctx.axes);
    let chrome_inline = edges.inline_chrome();
    let chrome_block = edges.block_chrome();
    let (chrome_main, chrome_cross) = if ctx.axes.row {
        (chrome_inline, chrome_block)
    } else {
        (chrome_block, chrome_inline)
    };
    let (assigned_main, assigned_cross) = if ctx.axes.row {
        (ctx.assigned_w, ctx.assigned_h)
    } else {
        (ctx.assigned_h, ctx.assigned_w)
    };
    let inner_main = assigned_main.unwrap_or_else(|| {
        let value = dim_to_border_box(size_main, cb_main, chrome_main, style.box_sizing)
            .unwrap_or_else(|| cb_main - margin_lateral(edges, ctx.axes));
        let (min, max) = item_min_max_main(style, ctx.axes, cb_main, edges);
        clamp(value, min, max)
    }) - chrome_main;
    let inner_cross = assigned_cross.unwrap_or_else(|| {
        let value = dim_to_border_box(size_cross, cb_cross, chrome_cross, style.box_sizing)
            .unwrap_or_else(|| cb_cross - margin_lateral_cross(edges, ctx.axes));
        let axes = Axis2 {
            row: !ctx.axes.row,
            reversed_main: false,
        };
        let (min, max) = item_min_max_main(style, axes, cb_cross, edges);
        clamp(value, min, max)
    }) - chrome_cross;
    (inner_main.max(0.0), inner_cross.max(0.0))
}

fn margin_lateral(edges: &NodeEdges, axes: Axis2) -> f32 {
    let m = edges.margin.or_zero();
    if axes.row {
        m.left + m.right
    } else {
        m.top + m.bottom
    }
}

fn margin_lateral_cross(edges: &NodeEdges, axes: Axis2) -> f32 {
    let m = edges.margin.or_zero();
    if axes.row {
        m.top + m.bottom
    } else {
        m.left + m.right
    }
}

pub(super) fn main_cross_size(style: &StyleData, axes: Axis2) -> (Dimension, Dimension) {
    if axes.row {
        (style.size.width, style.size.height)
    } else {
        (style.size.height, style.size.width)
    }
}

pub(crate) fn item_cross_basis(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    axes: Axis2,
    cb_main: f32,
    cb_cross: f32,
    edges: &NodeEdges,
) -> (Option<f32>, bool) {
    let chrome_cross = if axes.row {
        edges.block_chrome()
    } else {
        edges.inline_chrome()
    };
    let dim = if axes.row {
        style.size.height
    } else {
        style.size.width
    };
    let was_auto = matches!(dim, Dimension::Auto);
    let resolved = dim_to_border_box(dim, cb_cross, chrome_cross, style.box_sizing);
    let resolved = resolved.or_else(|| {
        if node.children().is_empty() {
            let measured = text_intrinsic(ctx, node, style)?;
            Some(if axes.row {
                measured.height + chrome_cross
            } else {
                measured.width + chrome_cross
            })
        } else {
            container_intrinsic_cross(ctx, node, cb_main, cb_cross, axes)
        }
    });
    (resolved, was_auto)
}

pub(crate) fn item_main_basis(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    axes: Axis2,
    cb_main: f32,
    edges: &NodeEdges,
) -> f32 {
    let chrome_main = if axes.row {
        edges.inline_chrome()
    } else {
        edges.block_chrome()
    };
    if let Some(v) = dim_to_border_box(style.flex_basis, cb_main, chrome_main, style.box_sizing) {
        return v.max(0.0);
    }
    let dim = if axes.row {
        style.size.width
    } else {
        style.size.height
    };
    if let Some(size) = dim_to_border_box(dim, cb_main, chrome_main, style.box_sizing) {
        return size.max(0.0);
    }
    if node.children().is_empty() {
        return text_intrinsic(ctx, node, style)
            .map_or(0.0, |measured| {
                if axes.row {
                    measured.width + chrome_main
                } else {
                    measured.height + chrome_main
                }
            })
            .max(0.0);
    }
    let (cb_main_axis, cb_cross_axis) = ctx.cb.along(axes);
    container_intrinsic_main(ctx, node, cb_main_axis, cb_cross_axis, axes)
        .unwrap_or(0.0)
        .max(0.0)
}

pub(crate) fn item_min_max_main(
    style: &StyleData,
    axes: Axis2,
    cb_main: f32,
    edges: &NodeEdges,
) -> (Option<f32>, Option<f32>) {
    let chrome_main = if axes.row {
        edges.inline_chrome()
    } else {
        edges.block_chrome()
    };
    let (mn, mx) = if axes.row {
        (style.min_size.width, style.max_size.width)
    } else {
        (style.min_size.height, style.max_size.height)
    };
    (
        dim_to_border_box(mn, cb_main, chrome_main, style.box_sizing),
        dim_to_border_box(mx, cb_main, chrome_main, style.box_sizing),
    )
}

pub(crate) fn automatic_min_main(
    style: &StyleData,
    axes: Axis2,
    content_basis: f32,
) -> Option<f32> {
    let (overflow_main, size_main) = if axes.row {
        (style.overflow.width, style.size.width)
    } else {
        (style.overflow.height, style.size.height)
    };
    if overflow_main == Overflow::Visible
        && style.flex_basis == Dimension::Auto
        && size_main == Dimension::Auto
    {
        Some(content_basis.max(0.0))
    } else {
        None
    }
}

pub(crate) fn align_self_of(style: &StyleData) -> Option<AlignSelf> {
    style.align_self
}
