use crate::layout::LayoutError;
use crate::layout::flex::{CbSize, Position2};
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{Dimension, Display, StyleData};

use super::placement::PlacedItem;
use super::sizing::item_min_max_main;
use super::{Axis2, FlexCtx, layout_root};

pub(super) fn write_items(
    ctx: &FlexCtx<'_, '_>,
    placed: &[PlacedItem],
    content_origin: Position2,
) -> Result<(), LayoutError> {
    for p in placed {
        let (x, y, w, h) = if ctx.axes.row {
            (
                content_origin.x + p.main_pos,
                content_origin.y + p.cross_pos,
                p.main_size,
                p.cross_size,
            )
        } else {
            (
                content_origin.x + p.cross_pos,
                content_origin.y + p.main_pos,
                p.cross_size,
                p.main_size,
            )
        };
        ctx.solver.write(p.id, ComputedBox { x, y, w, h });
        layout_item_subtree(ctx, p.id, Position2::new(x, y), w, h)?;
    }
    Ok(())
}

pub(crate) fn layout_item_subtree(
    ctx: &FlexCtx<'_, '_>,
    id: crate::layout::forest::LocalId,
    origin: Position2,
    w: f32,
    h: f32,
) -> Result<(), LayoutError> {
    let forest = ctx.solver.scope().forest();
    let node = forest
        .refer(id)
        .ok_or_else(|| LayoutError::LayoutInvalid(format!("stale flex item id {id:?}")))?;
    if node.children().is_empty() {
        return Ok(());
    }
    let style = node.style();
    let (assigned_w, assigned_h) = assigned_size(ctx, style, w, h);
    match style.display {
        Display::Flex => {
            let axes = Axis2::for_direction(style.flex_direction);
            let nested = FlexCtx {
                solver: ctx.solver,
                axes,
                cb: CbSize { w, h },
                suppress_stretch: ctx.suppress_stretch,
                assigned_w,
                assigned_h,
            };
            let edges = NodeEdges::build(style, w);
            let m = edges.margin.or_zero();
            let pre = Position2::new(origin.x - m.left, origin.y - m.top);
            layout_root(&nested, node, pre)
        }
        Display::Grid => {
            let scope = ctx.solver.scope();
            let nested = crate::layout::forest::MeasureScope::new(
                scope.forest(),
                scope.text(),
                scope.available(),
            );
            let edges = NodeEdges::build(style, w);
            let m = edges.margin.or_zero();
            let pre = crate::layout::block::Position2 {
                x: origin.x - m.left,
                y: origin.y - m.top,
            };
            crate::layout::grid::GridSolver::new(nested).solve_at(node, w, pre)
        }
        Display::Block | Display::None => {
            dispatch_block_subtree(ctx, node, style, origin, w, h, assigned_w)
        }
    }
}

fn dispatch_block_subtree(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    origin: Position2,
    w: f32,
    h: f32,
    assigned_w: Option<f32>,
) -> Result<(), LayoutError> {
    let scope = ctx.solver.scope();
    let nested =
        crate::layout::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
    let solver = crate::layout::block::BlockSolver::new(nested);
    let cb = crate::layout::style_data::Size {
        width: w,
        height: h,
    };
    let edges = NodeEdges::build(style, w);
    let m = edges.margin.or_zero();
    let pre = crate::layout::block::Position2 {
        x: origin.x - m.left,
        y: origin.y - m.top,
    };
    solver.solve_at_assigned(node, cb, pre, assigned_w)
}

pub(super) fn write_container(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    edges: &NodeEdges,
    border_origin: Position2,
    container_main: f32,
    placed: &[PlacedItem],
) {
    let (chrome_main, chrome_cross) = if ctx.axes.row {
        (edges.inline_chrome(), edges.block_chrome())
    } else {
        (edges.block_chrome(), edges.inline_chrome())
    };
    let style = node.style();
    let cross_explicit = match (
        ctx.axes.row,
        style.size.height,
        style.size.width,
        style.box_sizing,
    ) {
        (true, Dimension::Auto, _, _) | (false, _, Dimension::Auto, _) => None,
        (true, dim, _, sizing) => dim_to_border_box(dim, ctx.cb.h, chrome_cross, sizing),
        (false, _, dim, sizing) => dim_to_border_box(dim, ctx.cb.w, chrome_cross, sizing),
    };
    let auto_cross = placed
        .iter()
        .map(|p| p.cross_pos + p.cross_size)
        .fold(0.0_f32, f32::max)
        + chrome_cross;
    let axes = Axis2 {
        row: !ctx.axes.row,
        reversed_main: false,
    };
    let basis = if ctx.axes.row { ctx.cb.h } else { ctx.cb.w };
    let (min, max) = item_min_max_main(style, axes, basis, edges);
    let cross_size = clamp(cross_explicit.unwrap_or(auto_cross), min, max);
    let main_size = container_main + chrome_main;
    let (w, h) = if ctx.axes.row {
        (main_size, cross_size)
    } else {
        (cross_size, main_size)
    };
    let w = ctx.assigned_w.unwrap_or(w);
    let h = ctx.assigned_h.unwrap_or(h);
    ctx.solver.write(
        node.id(),
        ComputedBox {
            x: border_origin.x,
            y: border_origin.y,
            w,
            h,
        },
    );
}

fn assigned_size(
    ctx: &FlexCtx<'_, '_>,
    style: &StyleData,
    w: f32,
    h: f32,
) -> (Option<f32>, Option<f32>) {
    let probing = ctx.suppress_stretch;
    let assigned_w = (!probing
        && (style.size.width != Dimension::Auto || (style.flex_grow > 0.0 && ctx.axes.row)))
        .then_some(w);
    let assigned_h = (!probing
        && (style.size.height != Dimension::Auto || (style.flex_grow > 0.0 && !ctx.axes.row)))
        .then_some(h);
    (assigned_w, assigned_h)
}
