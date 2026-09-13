use crate::layout::flex::Position2;
use crate::layout::forest::NodeRef;
use crate::layout::resolve::{NodeEdges, dim_to_border_box};
use crate::layout::style_data::{Display, FlexWrap, Position, StyleData};

use super::{Axis2, FlexCtx, item_main_basis, layout_item_subtree, main_gap};

pub(super) fn container_intrinsic_cross(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    cb_main: f32,
    cb_cross: f32,
    axes: Axis2,
) -> Option<f32> {
    if let Some(extent) = ctx
        .solver
        .intrinsic_get(node.id(), cb_main, cb_cross, false)
    {
        return Some(extent);
    }
    let probe = FlexCtx {
        solver: ctx.solver,
        axes: ctx.axes,
        cb: ctx.cb,
        suppress_stretch: true,
        assigned_w: None,
        assigned_h: None,
    };
    let (w, h) = if axes.row {
        (cb_main, cb_cross)
    } else {
        (cb_cross, cb_main)
    };
    layout_item_subtree(&probe, node.id(), Position2::new(0.0, 0.0), w, h).ok()?;
    let extent = probed_content_extent(ctx, node, false, axes);
    ctx.solver
        .intrinsic_put(node.id(), cb_main, cb_cross, false, extent);
    Some(extent)
}

pub(super) fn container_intrinsic_main(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    cb_main: f32,
    cb_cross: f32,
    axes: Axis2,
) -> Option<f32> {
    if let Some(extent) = ctx.solver.intrinsic_get(node.id(), cb_main, cb_cross, true) {
        return Some(extent);
    }
    let style = node.style();
    let node_axes = Axis2::for_direction(style.flex_direction);
    let cheap = matches!(style.display, Display::Flex)
        && matches!(style.flex_wrap, FlexWrap::NoWrap)
        && node_axes.row == axes.row;
    let extent = if cheap {
        content_main_from_children(ctx, node, style, axes, cb_main, cb_cross)?
    } else {
        let (w, h) = if axes.row {
            (cb_main, cb_cross)
        } else {
            (cb_cross, cb_main)
        };
        let probe_ctx = FlexCtx {
            solver: ctx.solver,
            axes: ctx.axes,
            cb: ctx.cb,
            suppress_stretch: true,
            assigned_w: None,
            assigned_h: None,
        };
        layout_item_subtree(&probe_ctx, node.id(), Position2::new(0.0, 0.0), w, h).ok()?;
        probed_content_extent(ctx, node, true, axes)
    };
    ctx.solver
        .intrinsic_put(node.id(), cb_main, cb_cross, true, extent);
    Some(extent)
}

fn content_main_from_children(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    axes: Axis2,
    cb_main: f32,
    cb_cross: f32,
) -> Option<f32> {
    let edges = NodeEdges::build(style, ctx.cb.w);
    let chrome_main = if axes.row {
        edges.inline_chrome()
    } else {
        edges.block_chrome()
    };
    let forest = ctx.solver.scope().forest();
    let cb_inline = if axes.row { cb_main } else { cb_cross };
    let mut count = 0usize;
    let mut bases = 0.0_f32;
    for child_id in node.children() {
        let child = forest.refer(*child_id)?;
        let child_style = child.style();
        if matches!(child_style.display, Display::None)
            || matches!(child_style.position, Position::Absolute)
        {
            continue;
        }
        let child_edges = NodeEdges::build(child_style, cb_inline);
        let inner = item_main_basis(ctx, child, child_style, axes, cb_main, &child_edges);
        let m = child_edges.margin.or_zero();
        let margin_main = if axes.row {
            m.left + m.right
        } else {
            m.top + m.bottom
        };
        bases += inner + margin_main;
        count += 1;
    }
    let gap = main_gap(ctx, style, count);
    Some(bases + gap * (count.saturating_sub(1) as f32) + chrome_main)
}

pub(super) fn probed_content_extent(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    along_main: bool,
    axes: Axis2,
) -> f32 {
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    let horizontal = axes.row == along_main;
    let (chrome, lead) = if horizontal {
        (
            edges.inline_chrome(),
            edges.padding.left + edges.border.left,
        )
    } else {
        (edges.block_chrome(), edges.padding.top + edges.border.top)
    };
    let forest = ctx.solver.scope().forest();
    let far_edge = node
        .children()
        .iter()
        .filter_map(|child_id| forest.boxed(*child_id))
        .map(|cb| if horizontal { cb.x + cb.w } else { cb.y + cb.h })
        .fold(0.0_f32, f32::max);
    (far_edge - lead).max(0.0) + chrome
}

pub(super) fn text_intrinsic(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    axes: Axis2,
) -> Option<crate::layout::style_data::Size<f32>> {
    if !node.children().is_empty() {
        return None;
    }
    let known = crate::layout::style_data::Size {
        width: dim_to_border_box(
            style.size.width,
            if axes.row { ctx.cb.w } else { ctx.cb.h },
            0.0,
            style.box_sizing,
        ),
        height: dim_to_border_box(
            style.size.height,
            if axes.row { ctx.cb.h } else { ctx.cb.w },
            0.0,
            style.box_sizing,
        ),
    };
    ctx.solver.scope().measure_text(node, known, Some(ctx.cb.w))
}
