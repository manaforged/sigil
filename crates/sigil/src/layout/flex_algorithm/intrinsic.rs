use crate::layout::flex::{CbSize, Position2};
use crate::layout::forest::NodeRef;
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box};
use crate::layout::style_data::{
    AlignItems, Display, FlexWrap, Length, Overflow, Position, StyleData,
};

use super::sizing::{container_inner_size, item_cross_basis, item_min_max_main};
use super::{Axis2, FlexCtx, item_main_basis, layout_item_subtree, main_gap};

pub(super) fn min_content_width(ctx: &FlexCtx<'_, '_>, node: NodeRef<'_>) -> f32 {
    let style = node.style();
    if matches!(style.display, Display::None) || matches!(style.position, Position::Absolute) {
        return 0.0;
    }
    let edges = node.edges(ctx.cb.w);
    let chrome = edges.inline_chrome();
    let m = edges.margin.or_zero();
    let (min, max) = (
        dim_to_border_box(style.min_size.width, ctx.cb.w, chrome, style.box_sizing),
        dim_to_border_box(style.max_size.width, ctx.cb.w, chrome, style.box_sizing),
    );
    let sized = matches!(style.size.width, Length::Px(_))
        .then(|| dim_to_border_box(style.size.width, ctx.cb.w, chrome, style.box_sizing))
        .flatten();
    let inner = sized.unwrap_or_else(|| content_min_width(ctx, node, style) + chrome);
    clamp(inner, min, max).max(0.0) + m.left + m.right
}

fn content_min_width(ctx: &FlexCtx<'_, '_>, node: NodeRef<'_>, style: &StyleData) -> f32 {
    let scope = ctx.solver.scope();
    if node.children().is_empty() {
        return scope.min_content_width(node).unwrap_or(0.0);
    }
    if style.overflow.width != Overflow::Visible {
        return 0.0;
    }
    let forest = scope.forest();
    let widths = node
        .children()
        .iter()
        .filter_map(|child| forest.refer(*child))
        .filter(|child| {
            !matches!(child.style().display, Display::None)
                && !matches!(child.style().position, Position::Absolute)
        })
        .map(|child| min_content_width(ctx, child));
    let in_line = matches!(style.display, Display::Flex)
        && matches!(style.flex_wrap, FlexWrap::NoWrap)
        && Axis2::for_direction(style.flex_direction).row;
    if !in_line {
        return widths.fold(0.0, f32::max);
    }
    let (count, total) = widths.fold((0_usize, 0.0), |(count, total), width| {
        (count + 1, total + width)
    });
    let axes = Axis2::for_direction(style.flex_direction);
    total + main_gap(&FlexCtx { axes, ..*ctx }, style, count) * count.saturating_sub(1) as f32
}

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
    let extent = if let Some(extent) = leaf_column_width(ctx, node, axes, cb_main, cb_cross) {
        extent
    } else {
        let cb = CbSize::across(axes, cb_main, cb_cross);
        let probe = if axes.row {
            ctx.height_probe()
        } else {
            ctx.probe()
        };
        layout_item_subtree(&probe, node.id(), Position2::new(0.0, 0.0), cb.w, cb.h).ok()?;
        probed_content_extent(ctx, node, false, axes)
    };
    ctx.solver
        .intrinsic_put(node.id(), cb_main, cb_cross, false, extent);
    Some(extent)
}

fn leaf_column_width(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    axes: Axis2,
    cb_main: f32,
    cb_cross: f32,
) -> Option<f32> {
    let style = node.style();
    let own = Axis2::for_direction(style.flex_direction);
    let plain = !axes.row
        && !own.row
        && matches!(style.display, Display::Flex)
        && matches!(style.flex_wrap, FlexWrap::NoWrap)
        && matches!(style.size.width, Length::Auto)
        && matches!(
            style.align_items,
            None | Some(AlignItems::Start | AlignItems::Stretch)
        );
    if !plain {
        return None;
    }
    let probe = ctx.probe();
    let measured = inner_context(&probe, style, own, cb_main, cb_cross);
    let (inner_main, inner_cross) = measured.cb.along(own);
    let edges = NodeEdges::build(style, measured.cb.w);
    let outer = edges.margin.or_zero();
    if outer.left != 0.0
        || outer.right != 0.0
        || item_min_max_main(style, own.cross(), cb_cross, &edges) != (None, None)
    {
        return None;
    }
    let forest = ctx.solver.scope().forest();
    let mut widest = 0.0_f32;
    for child_id in node.children() {
        let child = forest.refer(*child_id)?;
        let child_style = child.style();
        if matches!(child_style.display, Display::None)
            || matches!(child_style.position, Position::Absolute)
        {
            continue;
        }
        let child_edges = child.edges(measured.cb.w);
        let m = child_edges.margin.or_zero();
        let flush = m.left == 0.0 && m.right == 0.0;
        if !child.children().is_empty() || child_style.align_self.is_some() || !flush {
            return None;
        }
        let (cross, _) = item_cross_basis(
            &measured,
            child,
            child_style,
            own,
            inner_main,
            inner_cross,
            &child_edges,
        );
        let (min, max) = item_min_max_main(child_style, own.cross(), inner_cross, &child_edges);
        widest = widest.max(clamp(cross.unwrap_or(0.0), min, max).max(0.0));
    }
    Some(widest + NodeEdges::build(style, ctx.cb.w).inline_chrome())
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
        let cb = CbSize::across(axes, cb_main, cb_cross);
        let probe = if axes.row {
            ctx.probe()
        } else {
            ctx.height_probe()
        };
        layout_item_subtree(&probe, node.id(), Position2::new(0.0, 0.0), cb.w, cb.h).ok()?;
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
    let (chrome_main, _) = axes.chrome(&edges);
    let measured = inner_context(ctx, style, axes, cb_main, cb_cross);
    let (inner_main, _) = measured.cb.along(axes);
    let forest = measured.solver.scope().forest();
    let cb_inline = measured.cb.w;
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
        let child_edges = child.edges(cb_inline);
        let inner = item_main_basis(
            &measured,
            child,
            child_style,
            axes,
            inner_main,
            &child_edges,
            true,
        );
        let m = child_edges.margin.or_zero();
        let margin_main = axes.pick(m.left + m.right, m.top + m.bottom);
        let (min, max) = item_min_max_main(child_style, axes, inner_main, &child_edges);
        bases += clamp(inner, min, max) + margin_main;
        count += 1;
    }
    let gap = main_gap(&measured, style, count);
    Some(bases + gap * (count.saturating_sub(1) as f32) + chrome_main)
}

fn inner_context<'s, 'a>(
    ctx: &FlexCtx<'s, 'a>,
    style: &StyleData,
    axes: Axis2,
    main: f32,
    cross: f32,
) -> FlexCtx<'s, 'a> {
    let mut measured = FlexCtx {
        axes,
        cb: CbSize::across(axes, main, cross),
        assigned_w: None,
        assigned_h: None,
        ..*ctx
    };
    let edges = NodeEdges::build(style, measured.cb.w);
    let (inner_main, inner_cross) = container_inner_size(&measured, style, &edges);
    measured.cb = CbSize::across(axes, inner_main, inner_cross);
    measured
}

pub(super) fn probed_content_extent(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    along_main: bool,
    axes: Axis2,
) -> f32 {
    let edges = node.edges(ctx.cb.w);
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
        .filter(|child_id| {
            forest.style_for(**child_id).is_some_and(|child_style| {
                child_style.display != Display::None && child_style.position != Position::Absolute
            })
        })
        .filter_map(|child_id| forest.boxed(*child_id))
        .map(|cb| if horizontal { cb.x + cb.w } else { cb.y + cb.h })
        .fold(0.0_f32, f32::max);
    (far_edge - lead).max(0.0) + chrome
}

pub(super) fn text_intrinsic(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
) -> Option<crate::layout::style_data::Axes<f32>> {
    if !node.children().is_empty() {
        return None;
    }
    let edges = node.edges(ctx.cb.w);
    let known = crate::layout::style_data::Axes {
        width: dim_to_border_box(
            style.size.width,
            ctx.cb.w,
            edges.inline_chrome(),
            style.box_sizing,
        ),
        height: dim_to_border_box(
            style.size.height,
            ctx.cb.h,
            edges.block_chrome(),
            style.box_sizing,
        ),
    };
    ctx.solver
        .scope()
        .measure_leaf_text(node, known, ctx.cb.w, &edges)
}
