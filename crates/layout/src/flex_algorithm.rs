//! Flex algorithm — CSS Flexible Box Layout §9 implementation.
//!
//! Step order follows the spec:
//!   1. Determine container main-size + cross-size from style.
//!   2. Resolve each item's flex base size + hypothetical main size.
//!   3. Collect items into flex lines (single line when `NoWrap`).
//!   4. Resolve flexible lengths per line (grow / shrink).
//!   5. Determine each line's cross size (max outer cross of items).
//!   6. Apply `align-content` to distribute lines on cross axis.
//!   7. Apply `align-items` / `align-self` to place each item in its line.
//!   8. Apply `justify-content` to place items along the main axis.
//!
//! Out of scope (defaults to `start`): baseline alignment, intrinsic
//! min-content resolution beyond `flex-basis: 0`. Subgrid + container
//! queries are explicitly not addressed.

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

use crate::LayoutError;
use crate::flex::{CbSize, FlexSolver, Position2};
use crate::flex_lines::{FlexItem, FlexLine, collect_lines};
use crate::forest::{ComputedBox, NodeRef};
use crate::resolve::{NodeEdges, clamp, dim_to_border_box, resolve_lp};
use crate::style_data::{
    AlignContent, AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentageAuto, Overflow, Position, StyleData,
};

/// Axis orientation flags derived from `FlexDirection` + `FlexWrap`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis2 {
    /// True when the main axis runs along the inline (width) axis.
    pub(crate) row: bool,
    /// True when main-axis order is reversed.
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

/// Per-pass context handed to the algorithm helpers.
pub(crate) struct FlexCtx<'s, 'a> {
    pub(crate) solver: &'s FlexSolver<'a>,
    pub(crate) axes: Axis2,
    pub(crate) cb: CbSize,
    /// True when this layout pass is the hypothetical inside
    /// `container_intrinsic_*` — `cross_size_for` then skips the
    /// `align-items: stretch` inflation so basis reads back the
    /// children's natural extent instead of a stretched value.
    pub(crate) suppress_stretch: bool,
    /// Extent the parent's *flexible-length* pass resolved for this node,
    /// along the parent's main axis. Exactly one of these is set.
    ///
    /// Without them `write_container` re-derives the box from this node's
    /// own style and content, discarding what the parent just resolved:
    /// a `grow(1)` item collapses to its content (or to a `width(0)`
    /// flex-basis) the moment it has children, since childless items
    /// return early from `layout_item_subtree` and never reach the
    /// overwrite.
    ///
    /// Set only for a growing item, and only on the axis the parent
    /// actually flexed. Deliberately *not* set for `align-items: stretch`:
    /// a stretched item may legitimately be smaller than the line, and
    /// forcing it to fill overflows its parent's border.
    pub(crate) assigned_w: Option<f32>,
    /// See [`Self::assigned_w`].
    pub(crate) assigned_h: Option<f32>,
}

/// Lay out a flex container rooted at `node`. Entry point.
pub(crate) fn layout_root(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    origin: Position2,
) -> Result<(), LayoutError> {
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    let m = edges.margin.or_zero();
    let (container_main, container_cross) = container_inner_size(ctx, style, &edges);
    // Border-box origin = origin + margin offset.
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
    let area = crate::flex_pack::Area {
        border_origin,
        content_origin,
        main: container_main,
        cross: container_cross,
    };
    if crate::flex_pack::pack_definite(ctx, node, &edges, &area)? {
        return Ok(());
    }
    let padding_box = crate::style_data::Size {
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
    // Auto-main sizing: a container whose main size is not DEFINITE (no
    // explicit main length, and no extent the parent's flex pass assigned)
    // sizes to its CONTENT on the main axis, not to the containing block.
    // Without this, an auto-height column inside a shorter parent (a scroll
    // viewport especially) fills the parent's main extent and then §9.7
    // shrinks its items to fit — crushing stacked rows. Sizing to content
    // makes `free == 0`, so nothing grows or shrinks and the container
    // overflows its parent (and scrolls, under a clipping ancestor) instead.
    // Items that fill are unaffected: `grow` distributes from a definite
    // container, and an explicit/assigned size keeps `main_definite`.
    let container_main = if main_is_definite(ctx, style)
        || !matches!(style.flex_wrap, crate::style_data::FlexWrap::NoWrap)
    {
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

/// `true` when the container's MAIN size is definite: an explicit main-axis
/// length (or `Fill`), or an extent the parent's flex pass assigned. When
/// false the main size is `auto` and shrinks to content.
fn main_is_definite(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> bool {
    let (size_main, _) = main_cross_size(style, ctx.axes);
    let assigned_main = if ctx.axes.row {
        ctx.assigned_w
    } else {
        ctx.assigned_h
    };
    assigned_main.is_some() || !matches!(size_main, Dimension::Auto)
}

/// Content-based main size of the container: the sum of the items' outer
/// main bases plus the gaps between them (single-line `NoWrap` shape).
fn content_main_size(ctx: &FlexCtx<'_, '_>, style: &StyleData, items: &[FlexItem]) -> f32 {
    let count = items.len();
    let gap = main_gap(ctx, style, count);
    let bases: f32 = items.iter().map(FlexItem::outer_main_basis).sum();
    bases + gap * (count.saturating_sub(1) as f32)
}

/// Resolved inner content area of the container (main, cross) in px.
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
    // A parent-flexed extent is authoritative for the WHOLE pass, not just
    // the box written at the end: this node's children are laid out against
    // these values, so patching only the final write would size the box
    // correctly while its contents had already resolved against the
    // pre-flex (often zero) extent.
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

/// Build the [`FlexItem`] vector for the in-flow children.
fn build_items(
    ctx: &FlexCtx<'_, '_>,
    parent: NodeRef<'_>,
    container_main: f32,
    container_cross: f32,
    padding_box: crate::style_data::Size<f32>,
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
            // Absolutes don't participate in flex flow; place against
            // the container's padding box.
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
    // Skip the order sort when every item has the default order (0) —
    // that's the overwhelming common case and sorting a 100k flat tree
    // by a key that is the same value across the board is pure
    // overhead. `order` is `i32`; we only have to sort when at least
    // one item declared a non-zero order.
    if items.iter().any(|i| i.order != 0) {
        items.sort_by_key(|i| i.order);
    }
    Ok(items)
}

/// Place an absolute-positioned child against the container's content box.
/// An auto axis anchored by its LEADING inset (or unpinned) fills the
/// containing block, matching `block::size_absolute`. An auto axis
/// anchored by its TRAILING inset shrink-wraps to content: resolving
/// `origin + cb - box - inset` against a containing-block-sized box lands
/// the leading edge at `-inset`, fully offscreen.
fn place_absolute(
    ctx: &FlexCtx<'_, '_>,
    child: NodeRef<'_>,
    cb_main: f32,
    cb_cross: f32,
    padding_box: crate::style_data::Size<f32>,
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
    // RC3: recurse into the absolute child's subtree so descendants
    // (esp. text leaves) get LayoutResults. Before this call, an
    // absolutely-positioned flex child wrote only its own box and left
    // descendants without rects — the renderer's scene-walk then
    // dropped any text descendant on the floor because it had no
    // LayoutResult to read. `block::place_absolute` already recurses
    // via `size_absolute → layout_children`; flex was the asymmetric
    // case. Mirrors the in-flow flex item path at `layout_item_subtree`.
    layout_item_subtree(ctx, child.id(), Position2::new(x, y), w, h)?;
    Ok(())
}

/// Resolved border-box size of an absolute child: an explicit size wins;
/// a trailing-anchored auto axis shrink-wraps to the probed content
/// extent; any other auto axis fills the containing block.
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
    // The probe re-lays-out the subtree, so only pay for it when an
    // auto axis is trailing-anchored and actually needs the content size.
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

/// True when an axis is anchored by its trailing inset alone (leading
/// `Auto`, trailing fixed).
fn trailing_anchored(leading: LengthPercentageAuto, trailing: LengthPercentageAuto) -> bool {
    matches!(leading, LengthPercentageAuto::Auto) && !matches!(trailing, LengthPercentageAuto::Auto)
}

/// Resolve one axis of an absolute child's size: explicit size wins;
/// a trailing-anchored auto axis shrink-wraps to the probed content
/// extent; anything else fills the containing block (pre-existing rule).
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

/// Shrink-to-fit content size of an absolute child: probe-lay out the
/// subtree at the origin with stretch suppressed and read the laid-out
/// content far edges, mirroring the `container_intrinsic_*` probes.
/// `None` when the probe fails — the caller falls back to the fill.
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
        // Measurement probe: shrink-to-fit is the wanted behaviour.
        assigned_w: None,
        assigned_h: None,
    };
    layout_item_subtree(&probe, child.id(), Position2::new(0.0, 0.0), cb_w, cb_h).ok()?;
    Some((
        probed_content_extent(ctx, child, ctx.axes.row, ctx.axes),
        probed_content_extent(ctx, child, !ctx.axes.row, ctx.axes),
    ))
}

/// Inputs to one axis of `inset` resolution. Mirrors `block::InsetAxis`
/// — kept in this module so the two solvers don't grow a cross-module
/// dependency.
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

/// Place each line's items in (main, cross) pixel coordinates.
fn place_lines(
    ctx: &FlexCtx<'_, '_>,
    mut lines: Vec<FlexLine>,
    style: &StyleData,
    container_main: f32,
    container_cross: f32,
) -> Vec<PlacedItem> {
    let line_count = lines.len();
    // A single line only stretches to the container's cross size when that
    // size is DEFINITE (explicit `height`/`width` on the cross axis, or an
    // extent the parent's flex pass assigned). When the cross size is auto,
    // the container shrinks to fit its content, so the line keeps its own
    // cross extent — otherwise the auto fallback to the containing block's
    // cross (in `container_inner_size`) makes `align-items: center` place the
    // item halfway down an oversized band and inflate the shrink-wrapped box.
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
    let cross_dir = matches!(style.flex_wrap, crate::style_data::FlexWrap::WrapReverse);
    let item_total: usize = lines.iter().map(|l| l.items.len()).sum();
    let mut placed = Vec::with_capacity(item_total);
    let mut cursor_cross = lines_origin;
    // Walk lines forward or reversed without allocating a `Vec<&mut FlexLine>`
    // intermediate. The common case is `cross_dir = false` (no
    // wrap-reverse); the index trick saves an allocation per layout_root
    // call and folds both directions into one loop.
    let line_total = lines.len();
    for i in 0..line_total {
        let idx = if cross_dir { line_total - 1 - i } else { i };
        let Some(line) = lines.get_mut(idx) else {
            continue;
        };
        // CSS rule: when there's exactly one flex line, the line takes
        // the container's full cross size — single-line containers
        // always stretch their cross axis. Multi-line: `align-content`
        // explicit `Stretch` distributes the free cross space across
        // lines; otherwise lines keep their natural cross size.
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

/// Resolved item with main/cross pixel position + outer box.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlacedItem {
    pub(crate) id: crate::forest::LocalId,
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
    // Walk items forward or reversed without allocating a `Vec<&FlexItem>`
    // intermediate. For wide_tree (1000-item flex line + 100k 100-item
    // sub-lines), this collect was the single largest remaining per-frame
    // allocation in place_line_items.
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

fn main_gap(ctx: &FlexCtx<'_, '_>, style: &StyleData, count: usize) -> f32 {
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

/// CSS §9.7 — resolve flexible lengths within a line by distributing
/// the free space across grow / shrink factors. Single pass — handles
/// the common shape; min/max clamps applied on the resolved size.
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
    // §9.7.4 is a LOOP, not a single pass. Distributing once and then
    // clamping strands the space a min/max-violating item refused: two
    // `grow:1` items sharing 300px where one caps at 110 must give the
    // other 190, not leave 40px unused.
    let growing = free > 0.0;
    let mut frozen = freeze_inflexible(&mut line.items, growing);
    while frozen.contains(&false) {
        let remaining = remaining_free(&line.items, &frozen, container_main, gap_total);
        if !flex_round(&mut line.items, &mut frozen, remaining, growing) {
            break;
        }
    }
}

/// This item's flex factor for the direction being resolved. Shrink is
/// weighted by the base size ("scaled flex shrink factor", §9.7.4).
fn flex_factor(item: &FlexItem, growing: bool) -> f32 {
    if growing {
        item.flex_grow.max(0.0)
    } else {
        item.flex_shrink.max(0.0) * item.flex_basis_resolved
    }
}

/// §9.7.2 — freeze every item that cannot flex in this direction, at its
/// clamped base size. Returns the frozen mask.
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

/// §9.7.4.b — space left once frozen items keep their resolved size and
/// unfrozen items are counted at their flex base size.
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

/// One §9.7.4 pass: distribute `remaining` across the unfrozen items,
/// clamp each to its min/max, then freeze the items whose clamp fought
/// the distribution in the same direction as the net violation. Their
/// surrendered space is what the next pass hands to the survivors.
///
/// Returns false when no further progress is possible.
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

/// §9.7.4.e — no net violation freezes everything; a positive net freezes
/// the min-clamped items, a negative net the max-clamped ones.
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
    // Guard against a pathological round that freezes nothing: without
    // this the loop could spin forever on NaN-ish factors.
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

/// Cross size of a line: the max of each item's outer cross size.
fn line_cross_size(line: &FlexLine) -> f32 {
    line.items
        .iter()
        .map(FlexItem::outer_cross)
        .fold(0.0_f32, f32::max)
}

/// Distribute free space along the cross axis between lines.
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

/// Distribute free space along the main axis between items.
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

/// Write resolved boxes into the solver's results map.
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
    id: crate::forest::LocalId,
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
    // What the parent's flexible-length pass resolved for THIS child,
    // along the parent's main axis. Shared by both dispatch branches.
    //
    // Suppressed during a hypothetical probe (`suppress_stretch`): the
    // intrinsic-basis pass feeds the item the CONTAINER's full extent as
    // available space, so re-deriving an assignment from this item's own
    // `flex_grow` would fake a flex result that never happened — the item
    // fills the probe width, its own grow children fill it in turn, and
    // the probe reads back the full containing block as the item's
    // "natural" basis. Every grow item then probes to the same inflated
    // basis and the line resolves equally (or into the shrink pass)
    // instead of at the items' own grow ratio.
    let probing = ctx.suppress_stretch;
    let assigned_w = (style.flex_grow > 0.0 && ctx.axes.row && !probing).then_some(w);
    let assigned_h = (style.flex_grow > 0.0 && !ctx.axes.row && !probing).then_some(h);
    // Q1 tertiary — per-subtree solver dispatch. The child's own
    // `display` decides which formatting context owns its children,
    // regardless of the parent's display. CSS interleaves block-in-flex
    // and flex-in-block freely.
    match style.display {
        Display::Flex => {
            let axes = Axis2::for_direction(style.flex_direction);
            let nested = FlexCtx {
                solver: ctx.solver,
                axes,
                cb: CbSize { w, h },
                suppress_stretch: ctx.suppress_stretch,
                // Carried on the axis the parent flexed so the nested
                // `write_container` cannot shrink-wrap it away. Applies
                // whether the child's own direction matches the parent's
                // (row-in-row: the parent flexed the child's main axis) or
                // is perpendicular (column-in-row: its cross axis).
                assigned_w,
                assigned_h,
            };
            // The flex placement pass already baked this node's margins into `origin`,
            // but `layout_root` re-applies them (it expects a PRE-margin origin). Feed the
            // pre-margin position so the margin lands exactly once — the same correction
            // `dispatch_block_subtree` makes for a block child. Without it a nested flex
            // container's margin is applied twice.
            let edges = NodeEdges::build(style, w);
            let m = edges.margin.or_zero();
            let pre = Position2::new(origin.x - m.left, origin.y - m.top);
            layout_root(&nested, node, pre)
        }
        // A grid child of a flex container runs its own grid solver at the
        // flex-resolved width; the flex placement pass already baked this
        // node's margin into `origin`, so feed the pre-margin position.
        Display::Grid => {
            let scope = ctx.solver.scope();
            let nested =
                crate::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
            let edges = NodeEdges::build(style, w);
            let m = edges.margin.or_zero();
            let pre = crate::block::Position2 {
                x: origin.x - m.left,
                y: origin.y - m.top,
            };
            crate::grid::GridSolver::new(nested).solve_at(node, w, pre)
        }
        // `Display::None` skipped upstream in `build_items`.
        Display::Block | Display::None => {
            dispatch_block_subtree(ctx, node, style, origin, w, h, assigned_w)
        }
    }
}

/// Dispatch a `Display::Block` (or Grid fallback) child of a flex
/// container to a sibling [`crate::block::BlockSolver`] sharing the
/// same scope. `solve_at` does NOT re-apply this node's margins — the
/// flex placement pass already baked them into `origin`, so we feed
/// the pre-margin position back.
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
    let nested = crate::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
    let solver = crate::block::BlockSolver::new(nested);
    let cb = crate::style_data::Size {
        width: w,
        height: h,
    };
    let edges = NodeEdges::build(style, w);
    let m = edges.margin.or_zero();
    let pre = crate::block::Position2 {
        x: origin.x - m.left,
        y: origin.y - m.top,
    };
    // Carry the flex-resolved width so a block-display item with
    // `grow(1).width(0)` is not re-derived back to zero.
    solver.solve_at_assigned(node, cb, pre, assigned_w)
}

/// Write the container's own resolved box. Main size = container's
/// inner-main; cross size = sum of placed cross extents (auto height).
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
    // With no explicit cross size, prefer the extent the parent's
    // flexible-length pass already resolved for this item over
    // shrink-wrapping to content. Without this, an item with `grow(1)`
    // collapses back to its content extent the moment it has children,
    // since childless items return early from `layout_item_subtree` and
    // never reach this overwrite.
    //
    let cross_size = cross_explicit.unwrap_or(auto_cross);
    let main_size = container_main + chrome_main;
    let (w, h) = if ctx.axes.row {
        (main_size, cross_size)
    } else {
        (cross_size, main_size)
    };
    // Precedence: the flexed extent wins over both the specified size and
    // the shrink-wrapped content. CSS Flexbox §9.7 — a flex item's *used*
    // main size is the result of resolving flexible lengths, overriding
    // its own `width`/`height`. `flex: 1 1 0` is authored
    // `grow(1).width(0)`, so honouring the literal 0 collapses the item.
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

/// Trivial pipe helper to keep `distribute_cross` flow readable.
trait Pipe<T> {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R;
}

impl<T> Pipe<T> for T {
    fn pipe<R>(self, f: impl FnOnce(T) -> R) -> R {
        f(self)
    }
}

// Re-exported helpers used by flex_lines.
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
    // Child's cross dimension always resolves against the parent's
    // cross-axis containing block — cb_cross under both row and column
    // directions (see `FlexItem::build`: cb_main/cb_cross track main/cross
    // axes, not horizontal/vertical). The prior column-direction branch
    // used cb_main, which silently substituted the parent's height for
    // its width, collapsing `fill_width()` on a column child to the
    // column's height and overlapping its children at x=0.
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
            // Container with children + auto cross: run a hypothetical
            // layout pass to learn the natural cross size. CSS flexbox
            // §9.2 step 3 — the item's hypothetical main / cross size
            // depends on its content; for an auto-cross item this is
            // the post-layout cross extent of its subtree.
            container_intrinsic_cross(ctx, node, cb_main, cb_cross, axes)
        }
    });
    (resolved, was_auto)
}

/// Compute the natural cross size of a flex-or-block item with
/// children by laying out its subtree at origin (0, 0) under the
/// container's containing block, then reading the resulting box's
/// cross dimension out of the forest. The box is overwritten later
/// during the real placement pass in `write_items`.
fn container_intrinsic_cross(
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
    // Same hypothetical-stretch concern as `container_intrinsic_main`
    // — feed children their natural cross extent, not the stretched one.
    let probe = FlexCtx {
        solver: ctx.solver,
        axes: ctx.axes,
        cb: ctx.cb,
        suppress_stretch: true,
        // Measurement probe: shrink-to-fit is the wanted behaviour.
        assigned_w: None,
        assigned_h: None,
    };
    let (w, h) = if axes.row {
        (cb_main, cb_cross)
    } else {
        (cb_cross, cb_main)
    };
    layout_item_subtree(&probe, node.id(), Position2::new(0.0, 0.0), w, h).ok()?;
    // Read the content far-edge along the cross axis, NOT the node's own
    // box. For a child whose flex-direction differs from the parent's, the
    // cross dimension is the child's MAIN axis, which `container_inner_size`
    // inflated to the available main — reading the box directly returned the
    // parent's full extent (a column card inside a row reported height 750
    // instead of its 80 px of content).
    let extent = probed_content_extent(ctx, node, false, axes);
    ctx.solver
        .intrinsic_put(node.id(), cb_main, cb_cross, false, extent);
    Some(extent)
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
    // Container with children + auto main + no flex-basis: mirror the
    // cross-axis fallback. CSS flexbox §9.2 step 3 — the item's
    // hypothetical main size is its content's natural main extent.
    //
    // `ctx.cb.along(axes)` rather than raw `(ctx.cb.w, ctx.cb.h)`:
    // `container_intrinsic_main` interprets its first two arguments as
    // `(cb_main, cb_cross)` — axis-relative — but `ctx.cb.{w,h}` is
    // axis-absolute (horizontal, vertical). For a row parent the two
    // happen to coincide; for a column parent they're swapped, so
    // the hypothetical subtree got laid out at (parent_height,
    // parent_width) instead of (parent_width, parent_height), and
    // the basis came back as the parent's width — collapsing every
    // text leaf in a `patterns::field` flex_column down to its
    // shrink-clamped remainder.
    let (cb_main_axis, cb_cross_axis) = ctx.cb.along(axes);
    container_intrinsic_main(ctx, node, cb_main_axis, cb_cross_axis, axes)
        .unwrap_or(0.0)
        .max(0.0)
}

/// Natural main extent of an auto-main flex item with children.
///
/// `NoWrap` flex whose main axis matches `axes` sums child outer bases,
/// gaps, and this node's main-axis chrome — the same formula
/// `content_main_size` uses after the real item pass. Wrap, a
/// perpendicular flex-direction, grid, and block still probe-layout
/// and read [`probed_content_extent`].
fn container_intrinsic_main(
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
        // Hypothetical pass: suppress `align-items: stretch` so children's
        // boxes report their natural cross extent. With stretch on, an
        // empty container child inflates to fill its parent's cross axis;
        // the OUTER read then sees that as the OUTER's "natural" main
        // extent and over-reports its basis by the full parent dimension.
        let probe_ctx = FlexCtx {
            solver: ctx.solver,
            axes: ctx.axes,
            cb: ctx.cb,
            suppress_stretch: true,
            // Measurement probe: shrink-to-fit is the wanted behaviour.
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

/// Sum of in-flow child outer main bases + gaps + this node's main chrome.
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

/// Read the natural extent of `node`'s already-probed subtree along one
/// axis: the far edge of its child boxes minus the leading padding/border,
/// plus this node's own chrome on that axis.
///
/// `along_main` selects the outer-axis dimension to read (`true` = the
/// parent's main axis, `false` = its cross). Both intrinsic probes share
/// this so they read CONTENT extent rather than the child box's own size —
/// which, for an auto-main flex container, was inflated to the available
/// main (`container_inner_size`). Reading the box directly therefore
/// over-reported a column child's height inside a row as the parent's full
/// height; the content far-edge is direction-agnostic and correct.
///
/// The hypothetical pass placed `node` at origin (0, 0), so its child
/// boxes sit at content-origin = padding-start + border-start in
/// viewport-absolute coordinates. We subtract that leading offset before
/// re-adding it as chrome — otherwise the leading padding double-counts.
fn probed_content_extent(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    along_main: bool,
    axes: Axis2,
) -> f32 {
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    // The requested axis is horizontal exactly when reading the parent's
    // main on a row, or the parent's cross on a column.
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

fn text_intrinsic(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    style: &StyleData,
    axes: Axis2,
) -> Option<crate::style_data::Size<f32>> {
    if !node.children().is_empty() {
        return None;
    }
    let known = crate::style_data::Size {
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
    // Text wraps to its containing block's horizontal extent (cb.w is
    // physical width regardless of flex direction).
    ctx.solver.scope().measure_text(node, known, Some(ctx.cb.w))
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

/// CSS Flexbox §4.5 automatic minimum size.
///
/// A flex item whose main-axis minimum is `auto` (no explicit `min-*`),
/// whose basis is content-derived (no explicit `flex-basis` or main size),
/// and whose main-axis overflow is `visible` must not shrink below its
/// content size. `content_basis` is that content main size — the resolved
/// flex base size, which for these items equals the measured content.
///
/// Without this floor the shrink pass can crush a content-sized item (a
/// text node especially) toward zero when its flex container is shorter
/// than the content, collapsing stacked text on top of itself. Items with
/// clipping/scrolling overflow, an explicit size, or an explicit basis are
/// intentionally excluded — they are free to shrink.
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
