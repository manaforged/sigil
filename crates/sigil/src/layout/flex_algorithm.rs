#![allow(
    dead_code,
    clippy::redundant_pub_crate,
    clippy::cast_precision_loss,
    clippy::suboptimal_flops,
    clippy::too_many_arguments,
    clippy::match_same_arms,
    clippy::missing_const_for_fn,
    clippy::or_fun_call,
    clippy::unnecessary_wraps
)]

use crate::layout::LayoutError;
use crate::layout::flex::{CbSize, FlexSolver, Position2};
use crate::layout::flex_lines::{FlexItem, collect_lines};
use crate::layout::forest::NodeRef;
use crate::layout::resolve::{NodeEdges, clamp};
use crate::layout::style_data::{Dimension, Display, FlexDirection, FlexWrap, Position, StyleData};

mod absolute;
mod flexible;
mod intrinsic;
mod placement;
mod sizing;
mod subtree;

use absolute::place_absolute;
use placement::place_lines;
pub(crate) use placement::{gap_along, main_gap};
use sizing::container_inner_size;
pub(crate) use sizing::{automatic_min_main, item_cross_basis, item_main_basis, item_min_max_main};
pub(crate) use subtree::layout_item_subtree;
use subtree::{write_container, write_items};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis2 {
    pub(crate) row: bool,
    pub(crate) reversed_main: bool,
}

impl Axis2 {
    pub(crate) const fn for_direction(direction: FlexDirection) -> Self {
        Self {
            row: matches!(direction, FlexDirection::Row | FlexDirection::RowReverse),
            reversed_main: matches!(
                direction,
                FlexDirection::RowReverse | FlexDirection::ColumnReverse
            ),
        }
    }

    pub(crate) const fn cross(self) -> Self {
        Self {
            row: !self.row,
            reversed_main: false,
        }
    }

    pub(crate) fn pick<T>(self, row: T, column: T) -> T {
        if self.row { row } else { column }
    }

    pub(crate) fn chrome(self, edges: &NodeEdges) -> (f32, f32) {
        self.pick(
            (edges.inline_chrome(), edges.block_chrome()),
            (edges.block_chrome(), edges.inline_chrome()),
        )
    }
}

pub(crate) struct FlexCtx<'s, 'a> {
    pub(crate) solver: &'s FlexSolver<'a>,
    pub(crate) axes: Axis2,
    pub(crate) cb: CbSize,
    pub(crate) suppress_stretch: bool,
    pub(crate) assigned_w: Option<f32>,
    pub(crate) assigned_h: Option<f32>,
}

impl FlexCtx<'_, '_> {
    pub(crate) const fn probe(&self) -> Self {
        Self {
            suppress_stretch: true,
            assigned_w: None,
            assigned_h: None,
            ..*self
        }
    }

    pub(crate) fn definite_along(&self, style: &StyleData, row: bool) -> bool {
        let (assigned, dim) = if row {
            (self.assigned_w, style.size.width)
        } else {
            (self.assigned_h, style.size.height)
        };
        assigned.is_some() || !matches!(dim, Dimension::Auto)
    }
}

pub(crate) fn layout_root(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    origin: Position2,
) -> Result<(), LayoutError> {
    let ctx = &FlexCtx {
        assigned_w: ctx.assigned_w.or_else(|| filled_width(ctx, node)),
        ..*ctx
    };
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    let m = edges.margin.or_zero();
    let (container_main, container_cross) = container_inner_size(ctx, style, &edges);
    let border_origin = Position2::new(origin.x + m.left, origin.y + m.top);
    let content_origin = Position2::new(
        border_origin.x + edges.border.left + edges.padding.left,
        border_origin.y + edges.border.top + edges.padding.top,
    );
    let content = CbSize::across(ctx.axes, container_main, container_cross);
    let area = crate::layout::flex_pack::Area {
        border_origin,
        content_origin,
        main: container_main,
        cross: container_cross,
    };
    if crate::layout::flex_pack::pack_definite(ctx, node, &edges, &area)? {
        return Ok(());
    }
    let padding_box = crate::layout::style_data::Size {
        width: content.w + edges.padding.left + edges.padding.right,
        height: content.h + edges.padding.top + edges.padding.bottom,
    };
    let items = build_items(
        ctx,
        node,
        container_main,
        container_cross,
        padding_box,
        border_origin,
    )?;
    let container_main = if ctx.definite_along(style, ctx.axes.row)
        || !matches!(style.flex_wrap, FlexWrap::NoWrap)
    {
        container_main
    } else {
        content_main_size(ctx, style, &items, &edges)
    };
    let lines = collect_lines(ctx, items, style, container_main);
    let placed = place_lines(ctx, lines, style, container_main, container_cross)?;
    write_items(ctx, &placed, content_origin)?;
    write_container(ctx, node, &edges, border_origin, container_main, &placed);
    Ok(())
}

fn filled_width(ctx: &FlexCtx<'_, '_>, node: NodeRef<'_>) -> Option<f32> {
    let definite = matches!(
        ctx.solver.scope().available().w,
        crate::layout::forest::SpaceConstraint::Definite(_)
    );
    (!ctx.suppress_stretch && definite && ctx.solver.fills_width(node)).then(|| {
        let m = NodeEdges::build(node.style(), ctx.cb.w).margin.or_zero();
        (ctx.cb.w - m.left - m.right).max(0.0)
    })
}

fn content_main_size(
    ctx: &FlexCtx<'_, '_>,
    style: &StyleData,
    items: &[FlexItem],
    edges: &NodeEdges,
) -> f32 {
    let count = items.len();
    let gap = main_gap(ctx, style, count);
    let bases: f32 = items
        .iter()
        .map(|item| {
            clamp(item.flex_basis_resolved, item.min_main, item.max_main)
                + item.margin_main_start
                + item.margin_main_end
        })
        .sum();
    let (chrome, _) = ctx.axes.chrome(edges);
    let (min, max) = item_min_max_main(style, ctx.axes, ctx.cb.along(ctx.axes).0, edges);
    let size = bases + gap * (count.saturating_sub(1) as f32) + chrome;
    (clamp(size, min, max) - chrome).max(0.0)
}

fn build_items(
    ctx: &FlexCtx<'_, '_>,
    parent: NodeRef<'_>,
    container_main: f32,
    container_cross: f32,
    padding_box: crate::layout::style_data::Size<f32>,
    padding_origin: Position2,
) -> Result<Vec<FlexItem>, LayoutError> {
    let forest = ctx.solver.scope().forest();
    let children = parent.children();
    let mut items = Vec::with_capacity(children.len());
    for child_id in children {
        let child = forest.refer(*child_id).ok_or_else(|| {
            LayoutError::LayoutInvalid(format!("stale flex child id {child_id:?}"))
        })?;
        let style = child.style();
        if matches!(style.display, Display::None) {
            continue;
        }
        if matches!(style.position, Position::Absolute) {
            place_absolute(
                ctx,
                child,
                container_main,
                container_cross,
                padding_box,
                padding_origin,
            )?;
            continue;
        }
        items.push(FlexItem::build(ctx, child, container_main, container_cross));
    }
    if items.iter().any(|i| i.order != 0) {
        items.sort_by_key(|i| i.order);
    }
    Ok(items)
}
