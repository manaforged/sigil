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
use crate::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Position, Rect, StyleData,
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
}

/// Resolved pixel edges (no `auto`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Edges {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl Edges {
    pub(crate) const fn inline(self) -> f32 {
        self.left + self.right
    }
    pub(crate) const fn block(self) -> f32 {
        self.top + self.bottom
    }
}

/// Resolved pixel edges with auto-aware semantics (margins).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AutoEdges {
    pub(crate) left: Option<f32>,
    pub(crate) right: Option<f32>,
    pub(crate) top: Option<f32>,
    pub(crate) bottom: Option<f32>,
}

impl AutoEdges {
    pub(crate) fn or_zero(self) -> Edges {
        Edges {
            left: self.left.unwrap_or(0.0),
            right: self.right.unwrap_or(0.0),
            top: self.top.unwrap_or(0.0),
            bottom: self.bottom.unwrap_or(0.0),
        }
    }
}

/// Resolved padding + border + margin for a node.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NodeEdges {
    pub(crate) padding: Edges,
    pub(crate) border: Edges,
    pub(crate) margin: AutoEdges,
}

impl NodeEdges {
    pub(crate) fn build(style: &StyleData, cb_inline: f32) -> Self {
        Self {
            padding: edges_lp(style.padding, cb_inline),
            border: edges_lp(style.border, cb_inline),
            margin: edges_lp_auto(style.margin, cb_inline),
        }
    }

    pub(crate) const fn inline_chrome(&self) -> f32 {
        self.padding.left + self.padding.right + self.border.left + self.border.right
    }

    pub(crate) const fn block_chrome(&self) -> f32 {
        self.padding.top + self.padding.bottom + self.border.top + self.border.bottom
    }
}

/// Resolve a non-auto length/percentage against a basis.
pub(crate) fn resolve_lp(lp: LengthPercentage, cb: f32) -> f32 {
    match lp {
        LengthPercentage::Px(v) => v,
        LengthPercentage::Percent(p) => p * cb,
    }
}

/// Resolve an auto-capable length/percentage.
pub(crate) fn resolve_lp_auto(lp: LengthPercentageAuto, cb: f32) -> Option<f32> {
    match lp {
        LengthPercentageAuto::Px(v) => Some(v),
        LengthPercentageAuto::Percent(p) => Some(p * cb),
        LengthPercentageAuto::Auto => None,
    }
}

/// Resolve a `Dimension` against a containing-block axis.
pub(crate) fn resolve_dim(dim: Dimension, cb: f32) -> Option<f32> {
    match dim {
        Dimension::Px(v) => Some(v),
        Dimension::Percent(p) => Some(p * cb),
        Dimension::Fill => Some(cb),
        Dimension::Auto => None,
    }
}

fn edges_lp(rect: Rect<LengthPercentage>, cb_inline: f32) -> Edges {
    Edges {
        left: resolve_lp(rect.left, cb_inline),
        right: resolve_lp(rect.right, cb_inline),
        top: resolve_lp(rect.top, cb_inline),
        bottom: resolve_lp(rect.bottom, cb_inline),
    }
}

fn edges_lp_auto(rect: Rect<LengthPercentageAuto>, cb_inline: f32) -> AutoEdges {
    AutoEdges {
        left: resolve_lp_auto(rect.left, cb_inline),
        right: resolve_lp_auto(rect.right, cb_inline),
        top: resolve_lp_auto(rect.top, cb_inline),
        bottom: resolve_lp_auto(rect.bottom, cb_inline),
    }
}

/// Clamp a value to an optional min/max pair; CSS rule — min wins.
pub(crate) fn clamp_minmax(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let after_max = max.map_or(value, |m| value.min(m));
    min.map_or(after_max, |m| after_max.max(m))
}

/// Convert a `Dimension` to a border-box pixel value under `box_sizing`.
pub(crate) fn dim_to_border_box(
    dim: Dimension,
    cb: f32,
    chrome: f32,
    sizing: BoxSizing,
) -> Option<f32> {
    let resolved = resolve_dim(dim, cb)?;
    match sizing {
        BoxSizing::ContentBox => Some(resolved + chrome),
        BoxSizing::BorderBox => Some(resolved),
    }
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
    let lines = collect_lines(ctx, items, style, container_main);
    let placed = place_lines(ctx, lines, style, container_main, container_cross);
    write_items(ctx, &placed, content_origin)?;
    write_container(ctx, node, &edges, border_origin, container_main, &placed);
    Ok(())
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
    let inner_main = dim_to_border_box(size_main, cb_main, chrome_main, style.box_sizing)
        .map_or(cb_main - margin_lateral(edges, ctx.axes), |v| v)
        - chrome_main;
    let inner_cross = dim_to_border_box(size_cross, cb_cross, chrome_cross, style.box_sizing)
        .map_or(cb_cross - margin_lateral_cross(edges, ctx.axes), |v| v)
        - chrome_cross;
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

/// Place an absolute-positioned child against the container's content
/// box. Sizing falls back to its style preferred size, then to zero.
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
    let w = dim_to_border_box(
        style.size.width,
        cb_w,
        edges.inline_chrome(),
        style.box_sizing,
    )
    .unwrap_or(0.0);
    let h = dim_to_border_box(
        style.size.height,
        cb_h,
        edges.block_chrome(),
        style.box_sizing,
    )
    .unwrap_or(0.0);
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
            container_cross.max(line.cross)
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
    if free > 0.0 {
        grow_items(&mut line.items, free);
    } else {
        shrink_items(&mut line.items, -free);
    }
}

fn grow_items(items: &mut [FlexItem], free: f32) {
    let total: f32 = items.iter().map(|i| i.flex_grow.max(0.0)).sum();
    if total <= f32::EPSILON {
        for item in items.iter_mut() {
            item.main_size = clamp_main(item, item.flex_basis_resolved);
        }
        return;
    }
    for item in items.iter_mut() {
        let share = free * (item.flex_grow.max(0.0) / total);
        let target = item.flex_basis_resolved + share;
        item.main_size = clamp_main(item, target);
    }
}

fn shrink_items(items: &mut [FlexItem], deficit: f32) {
    let total: f32 = items
        .iter()
        .map(|i| i.flex_shrink.max(0.0) * i.flex_basis_resolved)
        .sum();
    if total <= f32::EPSILON {
        for item in items.iter_mut() {
            item.main_size = clamp_main(item, item.flex_basis_resolved);
        }
        return;
    }
    for item in items.iter_mut() {
        let weight = item.flex_shrink.max(0.0) * item.flex_basis_resolved;
        let share = deficit * (weight / total);
        let target = item.flex_basis_resolved - share;
        item.main_size = clamp_main(item, target);
    }
}

fn clamp_main(item: &FlexItem, value: f32) -> f32 {
    clamp_minmax(value, item.min_main, item.max_main).max(0.0)
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

fn layout_item_subtree(
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
            };
            layout_root(&nested, node, origin)
        }
        // `Display::None` skipped upstream in `build_items`; `Grid`
        // falls back to BlockSolver until the owned grid solver lands
        // (post-G2, see docs/followups.md).
        Display::Block | Display::None | Display::Grid => {
            dispatch_block_subtree(ctx, node, style, origin, w, h)
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
    solver.solve_at(node, cb, pre)
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
    let cross_size = cross_explicit.unwrap_or(auto_cross);
    let main_size = container_main + chrome_main;
    let (w, h) = if ctx.axes.row {
        (main_size, cross_size)
    } else {
        (cross_size, main_size)
    };
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
    // Same hypothetical-stretch concern as `container_intrinsic_main`
    // — feed children their natural cross extent, not the stretched one.
    let probe = FlexCtx {
        solver: ctx.solver,
        axes: ctx.axes,
        cb: ctx.cb,
        suppress_stretch: true,
    };
    let (w, h) = if axes.row {
        (cb_main, cb_cross)
    } else {
        (cb_cross, cb_main)
    };
    layout_item_subtree(&probe, node.id(), Position2::new(0.0, 0.0), w, h).ok()?;
    let cb = probe.solver.scope().forest().boxed(node.id())?;
    Some(if axes.row { cb.h } else { cb.w })
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

/// Mirror of `container_intrinsic_cross` for the main axis. Run a
/// hypothetical layout pass under the container's containing block at
/// origin (0, 0), then read back the natural main extent by taking the
/// max far-edge of the laid-out children plus the container's own
/// main-axis chrome. The container's own box stores the CB-bound main
/// size (because `write_container` always writes `inner_main + chrome`
/// on the main axis), so we cannot read it back from the box directly
/// the way `container_intrinsic_cross` does — child boxes hold the real
/// content extent. Recursion bottoms out at leaves (where
/// `layout_item_subtree` returns early) and at children with definite
/// main sizes.
fn container_intrinsic_main(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    cb_main: f32,
    cb_cross: f32,
    axes: Axis2,
) -> Option<f32> {
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
    };
    layout_item_subtree(&probe_ctx, node.id(), Position2::new(0.0, 0.0), w, h).ok()?;
    let style = node.style();
    let edges = NodeEdges::build(style, ctx.cb.w);
    // The hypothetical layout placed `node` at origin (0, 0); its child
    // boxes sit at content-origin = padding-start + border-start, in
    // viewport-absolute coordinates. Subtract that leading offset
    // before re-adding it as part of chrome — otherwise the leading
    // padding is double-counted (a chip with `padding_x(8)` and text
    // "NEUTRAL" returned 24+text instead of 16+text, which collapsed
    // every chip in a flex_row to a zero-or-too-small basis and
    // overlapped them at x=0).
    let (chrome_main, lead_main) = if axes.row {
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
        .map(|cb| if axes.row { cb.x + cb.w } else { cb.y + cb.h })
        .fold(0.0_f32, f32::max);
    let inner_content = (far_edge - lead_main).max(0.0);
    Some(inner_content + chrome_main)
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
    ctx.solver.scope().measure_text(node, known)
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

pub(crate) fn align_self_of(style: &StyleData) -> Option<AlignSelf> {
    style.align_self
}
