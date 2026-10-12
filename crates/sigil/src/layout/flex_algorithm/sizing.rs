use crate::layout::forest::NodeRef;
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{Dimension, Overflow, StyleData};

use super::intrinsic::{container_intrinsic_cross, container_intrinsic_main, text_intrinsic};
use super::{Axis2, FlexCtx};

pub(super) fn container_inner_size(
    ctx: &FlexCtx<'_, '_>,
    style: &StyleData,
    edges: &NodeEdges,
) -> (f32, f32) {
    let (cb_main, cb_cross) = ctx.cb.along(ctx.axes);
    let axes = ctx.axes;
    let (size_main, size_cross) = axes.pick(
        (style.size.width, style.size.height),
        (style.size.height, style.size.width),
    );
    let (chrome_main, chrome_cross) = axes.chrome(edges);
    let (assigned_main, assigned_cross) = axes.pick(
        (ctx.assigned_w, ctx.assigned_h),
        (ctx.assigned_h, ctx.assigned_w),
    );
    let m = edges.margin.or_zero();
    let (margin_main, margin_cross) = axes.pick(
        (m.left + m.right, m.top + m.bottom),
        (m.top + m.bottom, m.left + m.right),
    );
    let inner_main = assigned_main.unwrap_or_else(|| {
        let value = dim_to_border_box(size_main, cb_main, chrome_main, style.box_sizing)
            .unwrap_or(cb_main - margin_main);
        let (min, max) = item_min_max_main(style, axes, cb_main, edges);
        clamp(value, min, max)
    }) - chrome_main;
    let inner_cross = assigned_cross.unwrap_or_else(|| {
        let value = dim_to_border_box(size_cross, cb_cross, chrome_cross, style.box_sizing)
            .unwrap_or(cb_cross - margin_cross);
        let (min, max) = item_min_max_main(style, axes.cross(), cb_cross, edges);
        clamp(value, min, max)
    }) - chrome_cross;
    (inner_main.max(0.0), inner_cross.max(0.0))
}

pub(super) fn container_cross_bounds(
    ctx: &FlexCtx<'_, '_>,
    style: &StyleData,
) -> (Option<f32>, Option<f32>) {
    let edges = NodeEdges::build(style, ctx.cb.w);
    let (_, cb_cross) = ctx.cb.along(ctx.axes);
    let (_, chrome) = ctx.axes.chrome(&edges);
    let (min, max) = item_min_max_main(style, ctx.axes.cross(), cb_cross, &edges);
    (min.map(|v| v - chrome), max.map(|v| v - chrome))
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
    let (_, chrome_cross) = axes.chrome(edges);
    let dim = axes.pick(style.size.height, style.size.width);
    let was_auto = matches!(dim, Dimension::Auto);
    let resolved = dim_to_border_box(dim, cb_cross, chrome_cross, style.box_sizing);
    let resolved = resolved.or_else(|| {
        if node.children().is_empty() {
            let measured = text_intrinsic(ctx, node, style)?;
            Some(axes.pick(measured.height, measured.width) + chrome_cross)
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
    intrinsic: bool,
) -> f32 {
    let (chrome_main, _) = axes.chrome(edges);
    if let Some(v) = dim_to_border_box(style.flex_basis, cb_main, chrome_main, style.box_sizing) {
        return v.max(0.0);
    }
    let dim = axes.pick(style.size.width, style.size.height);
    let sized = !(intrinsic && matches!(dim, Dimension::Fill | Dimension::Percent(_)));
    if sized && let Some(size) = dim_to_border_box(dim, cb_main, chrome_main, style.box_sizing) {
        return size.max(0.0);
    }
    if node.children().is_empty() {
        return text_intrinsic(ctx, node, style)
            .map_or(0.0, |measured| {
                axes.pick(measured.width, measured.height) + chrome_main
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
    let (chrome_main, _) = axes.chrome(edges);
    let (mn, mx) = axes.pick(
        (style.min_size.width, style.max_size.width),
        (style.min_size.height, style.max_size.height),
    );
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
    let (overflow_main, size_main) = axes.pick(
        (style.overflow.width, style.size.width),
        (style.overflow.height, style.size.height),
    );
    if overflow_main == Overflow::Visible
        && style.flex_basis == Dimension::Auto
        && size_main == Dimension::Auto
    {
        Some(content_basis.max(0.0))
    } else {
        None
    }
}
