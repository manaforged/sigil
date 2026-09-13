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
use crate::layout::flex_lines::{FlexItem, FlexLine, collect_lines};
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{NodeEdges, clamp, dim_to_border_box, resolve_lp};
use crate::layout::style_data::{
    AlignContent, AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentageAuto, Overflow, Position, StyleData,
};
mod intrinsic;

use intrinsic::{
    container_intrinsic_cross, container_intrinsic_main, probed_content_extent, text_intrinsic,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis2 {
    pub(crate) row: bool,
    pub(crate) reversed_main: bool,
}

impl Axis2 {
    pub(crate) const fn for_direction(direction: FlexDirection) -> Self {
        match direction {
            FlexDirection::Row => Self {
                row: true,
                reversed_main: false,
            },
            FlexDirection::RowReverse => Self {
                row: true,
                reversed_main: true,
            },
            FlexDirection::Column => Self {
                row: false,
                reversed_main: false,
            },
            FlexDirection::ColumnReverse => Self {
                row: false,
                reversed_main: true,
            },
        }
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

pub(crate) fn layout_root(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    origin: Position2,
) -> Result<(), LayoutError> {
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    let m = edges.margin.or_zero();
    let (container_main, container_cross) = container_inner_size(ctx, style, &edges);
    let border_origin = Position2::new(origin.x + m.left, origin.y + m.top);
    let content_origin = Position2::new(
        border_origin.x + edges.border.left + edges.padding.left,
        border_origin.y + edges.border.top + edges.padding.top,
    );
    let (content_w, content_h) = if ctx.axes.row {
        (container_main, container_cross)
    } else {
        (container_cross, container_main)
    };
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
        width: content_w + edges.padding.left + edges.padding.right,
        height: content_h + edges.padding.top + edges.padding.bottom,
    };
    let items = build_items(
        ctx,
        node,
        container_main,
        container_cross,
        padding_box,
        border_origin,
    )?;
    let container_main =
        if main_is_definite(ctx, style) || !matches!(style.flex_wrap, FlexWrap::NoWrap) {
            container_main
        } else {
            content_main_size(ctx, style, &items)
        };
    let lines = collect_lines(ctx, items, style, container_main);
    let placed = place_lines(ctx, lines, style, container_main, container_cross);
    write_items(ctx, &placed, content_origin)?;
    write_container(ctx, node, &edges, border_origin, container_main, &placed);
    Ok(())
}

fn main_is_definite(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> bool {
    let (size_main, _) = main_cross_size(style, ctx.axes);
    let assigned_main = if ctx.axes.row {
        ctx.assigned_w
    } else {
        ctx.assigned_h
    };
    assigned_main.is_some() || !matches!(size_main, Dimension::Auto)
}

fn content_main_size(ctx: &FlexCtx<'_, '_>, style: &StyleData, items: &[FlexItem]) -> f32 {
    let count = items.len();
    let gap = main_gap(ctx, style, count);
    let bases: f32 = items.iter().map(FlexItem::outer_main_basis).sum();
    bases + gap * (count.saturating_sub(1) as f32)
}

fn container_inner_size(ctx: &FlexCtx<'_, '_>, style: &StyleData, edges: &NodeEdges) -> (f32, f32) {
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
        dim_to_border_box(size_main, cb_main, chrome_main, style.box_sizing)
            .unwrap_or_else(|| cb_main - margin_lateral(edges, ctx.axes))
    }) - chrome_main;
    let inner_cross = assigned_cross.unwrap_or_else(|| {
        dim_to_border_box(size_cross, cb_cross, chrome_cross, style.box_sizing)
            .unwrap_or_else(|| cb_cross - margin_lateral_cross(edges, ctx.axes))
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

fn main_cross_size(style: &StyleData, axes: Axis2) -> (Dimension, Dimension) {
    if axes.row {
        (style.size.width, style.size.height)
    } else {
        (style.size.height, style.size.width)
    }
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

fn place_absolute(
    ctx: &FlexCtx<'_, '_>,
    child: NodeRef<'_>,
    cb_main: f32,
    cb_cross: f32,
    padding_box: crate::layout::style_data::Size<f32>,
    padding_origin: Position2,
) -> Result<(), LayoutError> {
    let style = child.style();
    let cb_w = if ctx.axes.row { cb_main } else { cb_cross };
    let cb_h = if ctx.axes.row { cb_cross } else { cb_main };
    let edges = NodeEdges::build(style, cb_w);
    let margins = edges.margin.or_zero();
    let (w, h) = absolute_size(ctx, child, style, cb_w, cb_h);
    let x = resolve_inset_axis(InsetAxis {
        leading: style.inset.left,
        trailing: style.inset.right,
        origin: padding_origin.x,
        cb_dim: padding_box.width,
        box_dim: w,
        margin_leading: margins.left,
        margin_trailing: margins.right,
    });
    let y = resolve_inset_axis(InsetAxis {
        leading: style.inset.top,
        trailing: style.inset.bottom,
        origin: padding_origin.y,
        cb_dim: padding_box.height,
        box_dim: h,
        margin_leading: margins.top,
        margin_trailing: margins.bottom,
    });
    ctx.solver.write(child.id(), ComputedBox { x, y, w, h });
    layout_item_subtree(ctx, child.id(), Position2::new(x, y), w, h)?;
    Ok(())
}

fn absolute_size(
    ctx: &FlexCtx<'_, '_>,
    child: NodeRef<'_>,
    style: &StyleData,
    cb_w: f32,
    cb_h: f32,
) -> (f32, f32) {
    let edges = NodeEdges::build(style, cb_w);
    let margins = edges.margin.or_zero();
    let explicit_w = dim_to_border_box(
        style.size.width,
        cb_w,
        edges.inline_chrome(),
        style.box_sizing,
    );
    let explicit_h = dim_to_border_box(
        style.size.height,
        cb_h,
        edges.block_chrome(),
        style.box_sizing,
    );
    let probing = (explicit_w.is_none() && trailing_anchored(style.inset.left, style.inset.right))
        || (explicit_h.is_none() && trailing_anchored(style.inset.top, style.inset.bottom));
    let fit = if probing {
        shrink_fit(ctx, child, style, cb_w, cb_h)
    } else {
        None
    };
    let w = axis_size(
        explicit_w,
        style.inset.left,
        style.inset.right,
        (cb_w - margins.left - margins.right).max(0.0),
        fit.map(|f| f.0),
    );
    let h = axis_size(
        explicit_h,
        style.inset.top,
        style.inset.bottom,
        cb_h,
        fit.map(|f| f.1),
    );
    (w, h)
}

fn trailing_anchored(leading: LengthPercentageAuto, trailing: LengthPercentageAuto) -> bool {
    matches!(leading, LengthPercentageAuto::Auto) && !matches!(trailing, LengthPercentageAuto::Auto)
}

fn axis_size(
    explicit: Option<f32>,
    leading: LengthPercentageAuto,
    trailing: LengthPercentageAuto,
    fill: f32,
    fit: Option<f32>,
) -> f32 {
    if let Some(v) = explicit {
        return v;
    }
    if trailing_anchored(leading, trailing) {
        fit.unwrap_or(fill)
    } else {
        fill
    }
}

fn shrink_fit(
    ctx: &FlexCtx<'_, '_>,
    child: NodeRef<'_>,
    style: &StyleData,
    cb_w: f32,
    cb_h: f32,
) -> Option<(f32, f32)> {
    let edges = NodeEdges::build(style, cb_w);
    if child.children().is_empty() {
        let measured = text_intrinsic(ctx, child, style, ctx.axes);
        return Some((
            measured.map_or(0.0, |m| m.width) + edges.inline_chrome(),
            measured.map_or(0.0, |m| m.height) + edges.block_chrome(),
        ));
    }
    let probe = FlexCtx {
        solver: ctx.solver,
        axes: ctx.axes,
        cb: ctx.cb,
        suppress_stretch: true,
        assigned_w: None,
        assigned_h: None,
    };
    layout_item_subtree(&probe, child.id(), Position2::new(0.0, 0.0), cb_w, cb_h).ok()?;
    Some((
        probed_content_extent(ctx, child, ctx.axes.row, ctx.axes),
        probed_content_extent(ctx, child, !ctx.axes.row, ctx.axes),
    ))
}

#[derive(Clone, Copy)]
struct InsetAxis {
    leading: LengthPercentageAuto,
    trailing: LengthPercentageAuto,
    origin: f32,
    cb_dim: f32,
    box_dim: f32,
    margin_leading: f32,
    margin_trailing: f32,
}

fn resolve_inset_axis(a: InsetAxis) -> f32 {
    match (a.leading, a.trailing) {
        (LengthPercentageAuto::Auto, t) if !matches!(t, LengthPercentageAuto::Auto) => {
            let off = inset_px(t, a.cb_dim);
            a.origin + a.cb_dim - a.box_dim - off - a.margin_trailing
        }
        (l, _) => a.origin + inset_px(l, a.cb_dim) + a.margin_leading,
    }
}

fn inset_px(value: LengthPercentageAuto, basis: f32) -> f32 {
    match value {
        LengthPercentageAuto::Auto => 0.0,
        LengthPercentageAuto::Px(v) => v,
        LengthPercentageAuto::Percent(p) => p * basis,
    }
}

fn place_lines(
    ctx: &FlexCtx<'_, '_>,
    mut lines: Vec<FlexLine>,
    style: &StyleData,
    container_main: f32,
    container_cross: f32,
) -> Vec<PlacedItem> {
    let line_count = lines.len();
    let (_, size_cross) = main_cross_size(style, ctx.axes);
    let assigned_cross = if ctx.axes.row {
        ctx.assigned_h
    } else {
        ctx.assigned_w
    };
    let cross_definite = assigned_cross.is_some() || !matches!(size_cross, Dimension::Auto);
    for line in &mut lines {
        resolve_flexible(line, container_main);
        line.cross = line_cross_size(line);
    }
    let total_lines_cross: f32 = lines.iter().map(|l| l.cross).sum();
    let cross_free = (container_cross - total_lines_cross).max(0.0);
    let (lines_origin, lines_gap) =
        distribute_cross(style.align_content, cross_free, line_count, container_cross);
    let cross_dir = matches!(style.flex_wrap, FlexWrap::WrapReverse);
    let item_total: usize = lines.iter().map(|l| l.items.len()).sum();
    let mut placed = Vec::with_capacity(item_total);
    let mut cursor_cross = lines_origin;
    let line_total = lines.len();
    for i in 0..line_total {
        let idx = if cross_dir { line_total - 1 - i } else { i };
        let Some(line) = lines.get_mut(idx) else {
            continue;
        };
        let line_cross = if line_count == 1 {
            if cross_definite {
                container_cross.max(line.cross)
            } else {
                line.cross
            }
        } else if matches!(style.align_content, Some(AlignContent::Stretch)) && line_count > 0 {
            container_cross / (line_count as f32).max(1.0)
        } else {
            line.cross
        };
        place_line_items(
            ctx,
            line,
            style,
            container_main,
            cursor_cross,
            line_cross,
            &mut placed,
        );
        cursor_cross += line_cross + lines_gap;
    }
    placed
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PlacedItem {
    pub(crate) id: crate::layout::forest::LocalId,
    pub(crate) main_pos: f32,
    pub(crate) cross_pos: f32,
    pub(crate) main_size: f32,
    pub(crate) cross_size: f32,
    pub(crate) margin_main_start: f32,
    pub(crate) margin_cross_start: f32,
}

fn place_line_items(
    ctx: &FlexCtx<'_, '_>,
    line: &FlexLine,
    style: &StyleData,
    container_main: f32,
    line_origin_cross: f32,
    line_cross: f32,
    out: &mut Vec<PlacedItem>,
) {
    let item_count = line.items.len();
    let item_main_sum: f32 = line.items.iter().map(FlexItem::outer_main).sum();
    let gap = main_gap(ctx, style, item_count);
    let total_main = item_main_sum + gap * (item_count.saturating_sub(1) as f32);
    let free = (container_main - total_main).max(0.0);
    let (main_origin, main_gap_extra) =
        distribute_main(style.justify_content, free, item_count, container_main);
    let mut cursor = main_origin;
    let reversed = ctx.axes.reversed_main;
    for i in 0..item_count {
        let idx = if reversed { item_count - 1 - i } else { i };
        let Some(item) = line.items.get(idx) else {
            continue;
        };
        let cross_offset = cross_offset_for(item, style.align_items, line_cross);
        let placed = PlacedItem {
            id: item.id,
            main_pos: cursor + item.margin_main_start(),
            cross_pos: line_origin_cross + cross_offset + item.margin_cross_start(),
            main_size: item.main_size,
            cross_size: cross_size_for(item, style.align_items, line_cross, ctx.suppress_stretch),
            margin_main_start: item.margin_main_start(),
            margin_cross_start: item.margin_cross_start(),
        };
        out.push(placed);
        cursor += item.outer_main() + gap + main_gap_extra;
    }
}

pub(crate) fn main_gap(ctx: &FlexCtx<'_, '_>, style: &StyleData, count: usize) -> f32 {
    if count <= 1 {
        return 0.0;
    }
    let (cb_w, cb_h) = (ctx.cb.w, ctx.cb.h);
    if ctx.axes.row {
        resolve_lp(style.gap.width, cb_w)
    } else {
        resolve_lp(style.gap.height, cb_h)
    }
}

fn resolve_flexible(line: &mut FlexLine, container_main: f32) {
    let used_basis: f32 = line.items.iter().map(FlexItem::outer_main_basis).sum();
    let gap_total = line.gap * (line.items.len().saturating_sub(1) as f32);
    let free = container_main - used_basis - gap_total;
    if free.abs() < f32::EPSILON || line.items.is_empty() || container_main <= f32::EPSILON {
        for item in &mut line.items {
            item.main_size = clamp_main(item, item.flex_basis_resolved);
        }
        return;
    }
    let growing = free > 0.0;
    let mut frozen = freeze_inflexible(&mut line.items, growing);
    while frozen.contains(&false) {
        let remaining = remaining_free(&line.items, &frozen, container_main, gap_total);
        if !flex_round(&mut line.items, &mut frozen, remaining, growing) {
            break;
        }
    }
}

fn flex_factor(item: &FlexItem, growing: bool) -> f32 {
    if growing {
        item.flex_grow.max(0.0)
    } else {
        item.flex_shrink.max(0.0) * item.flex_basis_resolved
    }
}

fn freeze_inflexible(items: &mut [FlexItem], growing: bool) -> Vec<bool> {
    items
        .iter_mut()
        .map(|item| {
            let inflexible = flex_factor(item, growing) <= f32::EPSILON;
            item.main_size = if inflexible {
                clamp_main(item, item.flex_basis_resolved)
            } else {
                item.flex_basis_resolved
            };
            inflexible
        })
        .collect()
}

fn remaining_free(items: &[FlexItem], frozen: &[bool], container_main: f32, gap_total: f32) -> f32 {
    let used: f32 = items
        .iter()
        .zip(frozen)
        .map(|(item, &is_frozen)| {
            let base = if is_frozen {
                item.main_size
            } else {
                item.flex_basis_resolved
            };
            base + item.margin_main_start() + item.margin_main_end()
        })
        .sum();
    container_main - used - gap_total
}

fn flex_round(items: &mut [FlexItem], frozen: &mut [bool], remaining: f32, growing: bool) -> bool {
    let total: f32 = items
        .iter()
        .zip(frozen.iter())
        .filter(|(_, f)| !**f)
        .map(|(item, _)| flex_factor(item, growing))
        .sum();
    if total <= f32::EPSILON {
        for (item, is_frozen) in items.iter_mut().zip(frozen.iter_mut()) {
            if !*is_frozen {
                item.main_size = clamp_main(item, item.flex_basis_resolved);
                *is_frozen = true;
            }
        }
        return false;
    }
    let mut violations = vec![0.0_f32; items.len()];
    let mut net = 0.0_f32;
    for ((item, &is_frozen), violation) in items
        .iter_mut()
        .zip(frozen.iter())
        .zip(violations.iter_mut())
    {
        if is_frozen {
            continue;
        }
        let target = item.flex_basis_resolved + remaining * (flex_factor(item, growing) / total);
        item.main_size = clamp_main(item, target);
        *violation = item.main_size - target;
        net += *violation;
    }
    freeze_violations(frozen, &violations, net)
}

fn freeze_violations(frozen: &mut [bool], violations: &[f32], net: f32) -> bool {
    let mut progressed = false;
    for (is_frozen, &violation) in frozen.iter_mut().zip(violations) {
        if *is_frozen {
            continue;
        }
        let freeze = if net.abs() < f32::EPSILON {
            true
        } else if net > 0.0 {
            violation > 0.0
        } else {
            violation < 0.0
        };
        if freeze {
            *is_frozen = true;
            progressed = true;
        }
    }
    if !progressed {
        for f in frozen.iter_mut() {
            *f = true;
        }
    }
    progressed
}

fn clamp_main(item: &FlexItem, value: f32) -> f32 {
    clamp(value, item.min_main, item.max_main).max(0.0)
}

fn line_cross_size(line: &FlexLine) -> f32 {
    line.items
        .iter()
        .map(FlexItem::outer_cross)
        .fold(0.0_f32, f32::max)
}

fn distribute_cross(
    align: Option<AlignContent>,
    free: f32,
    line_count: usize,
    container_cross: f32,
) -> (f32, f32) {
    if line_count == 0 || free <= 0.0 {
        let origin = match align {
            Some(AlignContent::End) => free.max(0.0),
            Some(AlignContent::Center) => free.max(0.0) / 2.0,
            _ => 0.0,
        };
        return (origin, 0.0);
    }
    match align.unwrap_or(AlignContent::Stretch) {
        AlignContent::Start | AlignContent::Stretch => (0.0, 0.0),
        AlignContent::End => (free, 0.0),
        AlignContent::Center => (free / 2.0, 0.0),
        AlignContent::SpaceBetween => {
            if line_count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free / (line_count - 1) as f32)
            }
        }
        AlignContent::SpaceAround => {
            let g = free / line_count as f32;
            (g / 2.0, g)
        }
        AlignContent::SpaceEvenly => {
            let g = free / (line_count + 1) as f32;
            (g, g)
        }
    }
    .pipe(|(o, g)| {
        let _ = container_cross;
        (o, g)
    })
}

fn distribute_main(
    align: Option<JustifyContent>,
    free: f32,
    item_count: usize,
    container_main: f32,
) -> (f32, f32) {
    let _ = container_main;
    if item_count == 0 {
        return (0.0, 0.0);
    }
    if free <= 0.0 {
        let origin = match align {
            Some(JustifyContent::End) => 0.0,
            Some(JustifyContent::Center) => 0.0,
            _ => 0.0,
        };
        return (origin, 0.0);
    }
    match align.unwrap_or(JustifyContent::Start) {
        JustifyContent::Start | JustifyContent::Stretch => (0.0, 0.0),
        JustifyContent::End => (free, 0.0),
        JustifyContent::Center => (free / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if item_count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free / (item_count - 1) as f32)
            }
        }
        JustifyContent::SpaceAround => {
            let g = free / item_count as f32;
            (g / 2.0, g)
        }
        JustifyContent::SpaceEvenly => {
            let g = free / (item_count + 1) as f32;
            (g, g)
        }
    }
}

fn cross_offset_for(item: &FlexItem, container: Option<AlignItems>, line_cross: f32) -> f32 {
    let align = item.align_self.or(container).unwrap_or(AlignItems::Stretch);
    let extra = (line_cross - item.outer_cross()).max(0.0);
    match align {
        AlignItems::Start | AlignItems::Baseline | AlignItems::Stretch => 0.0,
        AlignItems::End => extra,
        AlignItems::Center => extra / 2.0,
    }
}

fn cross_size_for(
    item: &FlexItem,
    container: Option<AlignItems>,
    line_cross: f32,
    suppress_stretch: bool,
) -> f32 {
    let align = item.align_self.or(container).unwrap_or(AlignItems::Stretch);
    if !suppress_stretch && matches!(align, AlignItems::Stretch) && item.cross_size_was_auto {
        (line_cross - item.margin_cross_start() - item.margin_cross_end()).max(0.0)
    } else {
        item.cross_size
    }
}

fn write_items(
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
    let probing = ctx.suppress_stretch;
    let assigned_w = (style.flex_grow > 0.0 && ctx.axes.row && !probing).then_some(w);
    let assigned_h = (style.flex_grow > 0.0 && !ctx.axes.row && !probing).then_some(h);
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

fn write_container(
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
    let cross_size = cross_explicit.unwrap_or(auto_cross);
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

trait Pipe<T> {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R;
}

impl<T> Pipe<T> for T {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R {
        f(self)
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
            let measured = text_intrinsic(ctx, node, style, axes)?;
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
        return text_intrinsic(ctx, node, style, axes)
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
