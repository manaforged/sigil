use crate::Error;
use crate::layout::flex::{CbSize, Position2};
use crate::layout::forest::{ComputedBox, NodeRef, SolvedKey};
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{Axes, Display, Length, Position};

use super::placement::PlacedItem;
use super::sizing::item_min_max_main;
use super::{Axis2, FlexCtx, layout_root};

pub(super) fn write_items(
    ctx: &FlexCtx<'_, '_>,
    placed: &[PlacedItem],
    content_origin: Position2,
) -> Result<(), Error> {
    for p in placed {
        let pos = CbSize::across(ctx.axes, p.main_pos, p.cross_pos);
        let CbSize { w, h } = CbSize::across(ctx.axes, p.main_size, p.cross_size);
        let (x, y) = (content_origin.x + pos.w, content_origin.y + pos.h);
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
) -> Result<(), Error> {
    let scope = ctx.solver.scope();
    let forest = scope.forest();
    let node = forest
        .refer(id)
        .ok_or_else(|| Error::layout(format!("stale flex item id {id:?}")))?;
    if node.children().is_empty() {
        return Ok(());
    }
    let style = node.style();
    let (assigned_w, assigned_h) = assigned_size(ctx, node, w, h);
    let solved_key = if ctx.suppress_stretch || ctx.measures_height {
        forest.forget_solved_subtree(id);
        None
    } else {
        let key = SolvedKey::new(
            (w, h),
            (assigned_w, assigned_h),
            scope.available(),
            scope.text().font_generation(),
        );
        if forest.replay_subtree(id, key, (origin.x, origin.y)) {
            return Ok(());
        }
        Some(key)
    };
    match style.display {
        Display::Flex => {
            dispatch_flex_subtree(ctx, node, origin, (w, h), (assigned_w, assigned_h))?;
        }
        Display::Grid => {
            let nested =
                crate::layout::forest::MeasureScope::new(forest, scope.text(), scope.available());
            let pre = block_origin(style, origin, w);
            crate::layout::grid::GridSolver::new(nested).solve_at(node, w, pre)?;
        }
        Display::Block | Display::None => {
            dispatch_block_subtree(ctx, node, origin, (w, h), (assigned_w, assigned_h))?;
        }
    }
    if let Some(key) = solved_key {
        forest.store_solved(id, key, (origin.x, origin.y));
    }
    Ok(())
}

fn dispatch_flex_subtree(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    origin: Position2,
    (w, h): (f32, f32),
    (assigned_w, assigned_h): (Option<f32>, Option<f32>),
) -> Result<(), Error> {
    let style = node.style();
    let nested = FlexCtx {
        solver: ctx.solver,
        axes: Axis2::for_direction(style.flex_direction),
        cb: CbSize { w, h },
        suppress_stretch: ctx.suppress_stretch,
        measures_height: ctx.measures_height,
        assigned_w,
        assigned_h,
    };
    let m = NodeEdges::build(style, w).margin.or_zero();
    layout_root(
        &nested,
        node,
        Position2::new(origin.x - m.left, origin.y - m.top),
    )
}

fn dispatch_block_subtree(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    origin: Position2,
    (w, h): (f32, f32),
    (assigned_w, assigned_h): (Option<f32>, Option<f32>),
) -> Result<(), Error> {
    let style = node.style();
    let scope = ctx.solver.scope();
    let nested =
        crate::layout::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
    let solver = crate::layout::block::BlockSolver::new(nested);
    let cb = Axes {
        width: w,
        height: h,
    };
    solver.solve_at_assigned(
        node,
        cb,
        block_origin(style, origin, w),
        Axes {
            width: assigned_w,
            height: assigned_h,
        },
    )
}

pub(super) fn write_container(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    edges: &NodeEdges,
    border_origin: Position2,
    container_main: f32,
    placed: &[PlacedItem],
) {
    let (chrome_main, chrome_cross) = ctx.axes.chrome(edges);
    let style = node.style();
    let cross_explicit = match (
        ctx.axes.row,
        style.size.height,
        style.size.width,
        style.box_sizing,
    ) {
        (true, Length::Auto, _, _) | (false, _, Length::Auto, _) => None,
        (true, dim, _, sizing) => dim_to_border_box(dim, ctx.cb.h, chrome_cross, sizing),
        (false, _, dim, sizing) => dim_to_border_box(dim, ctx.cb.w, chrome_cross, sizing),
    };
    let auto_cross = placed
        .iter()
        .map(|p| p.cross_pos + p.cross_size)
        .fold(0.0_f32, f32::max)
        + chrome_cross;
    let basis = ctx.axes.pick(ctx.cb.h, ctx.cb.w);
    let (min, max) = item_min_max_main(style, ctx.axes.cross(), basis, edges);
    let cross_size = clamp(cross_explicit.unwrap_or(auto_cross), min, max);
    let size = CbSize::across(ctx.axes, container_main + chrome_main, cross_size);
    ctx.solver.write(
        node.id(),
        ComputedBox {
            x: border_origin.x,
            y: border_origin.y,
            w: ctx.assigned_w.unwrap_or(size.w),
            h: ctx.assigned_h.unwrap_or(size.h),
        },
    );
}

fn block_origin(
    style: &crate::layout::style_data::StyleData,
    origin: Position2,
    w: f32,
) -> crate::layout::block::Position2 {
    let m = NodeEdges::build(style, w).margin.or_zero();
    crate::layout::block::Position2 {
        x: origin.x - m.left,
        y: origin.y - m.top,
    }
}

fn assigned_size(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    w: f32,
    h: f32,
) -> (Option<f32>, Option<f32>) {
    let style = node.style();
    let probing = ctx.suppress_stretch;
    let in_flow = style.position != Position::Absolute;
    let sized = style.size.width != Length::Auto
        || (style.flex_grow > 0.0 && ctx.axes.row)
        || ctx.solver.fills_width(node);
    let stretched = (style.stretch_width || ctx.solver.fills_width(node))
        && in_flow
        && (!probing || ctx.measures_height);
    let assigned_w = ((!probing && sized) || stretched).then_some(w);
    let assigned_h = (!probing
        && (style.size.height != Length::Auto
            || (in_flow && ctx.axes.row)
            || style.flex_grow > 0.0))
        .then_some(h);
    (assigned_w, assigned_h)
}
