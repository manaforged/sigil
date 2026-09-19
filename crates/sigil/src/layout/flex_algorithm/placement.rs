use crate::layout::LayoutError;
use crate::layout::flex::CbSize;
use crate::layout::flex_lines::{FlexItem, FlexLine};
use crate::layout::resolve::{NodeEdges, clamp, resolve_lp};
use crate::layout::style_data::{
    AlignContent, AlignItems, Dimension, FlexWrap, JustifyContent, StyleData,
};

use super::FlexCtx;
use super::flexible::resolve_flexible;
use super::sizing::{item_cross_basis, main_cross_size};

pub(super) fn place_lines(
    ctx: &FlexCtx<'_, '_>,
    mut lines: Vec<FlexLine>,
    style: &StyleData,
    container_main: f32,
    container_cross: f32,
) -> Result<Vec<PlacedItem>, LayoutError> {
    let line_count = lines.len();
    let cross_definite = cross_is_definite(ctx, style);
    for line in &mut lines {
        resolve_flexible(line, container_main);
        resolve_cross_sizes(ctx, line, container_cross)?;
        line.cross = line_cross_size(line);
    }
    let total_lines_cross: f32 = lines.iter().map(|l| l.cross).sum();
    let gap = cross_gap(ctx, style);
    let total_gaps = gap * line_count.saturating_sub(1) as f32;
    let cross_free = if cross_definite {
        (container_cross - total_lines_cross - total_gaps).max(0.0)
    } else {
        0.0
    };
    let (lines_origin, lines_gap) = distribute_cross(style.align_content, cross_free, line_count);
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
        let line_cross = aligned_line_cross(
            line.cross,
            line_count,
            cross_definite,
            container_cross,
            cross_free,
            style.align_content,
        );
        place_line_items(
            ctx,
            line,
            style,
            container_main,
            cursor_cross,
            line_cross,
            &mut placed,
        );
        cursor_cross += line_cross + gap + lines_gap;
    }
    Ok(placed)
}

fn resolve_cross_sizes(
    ctx: &FlexCtx<'_, '_>,
    line: &mut FlexLine,
    container_cross: f32,
) -> Result<(), LayoutError> {
    if !ctx.axes.row {
        return Ok(());
    }
    let forest = ctx.solver.scope().forest();
    for item in &mut line.items {
        if !item.cross_size_was_auto {
            continue;
        }
        let node = forest.refer(item.id).ok_or_else(|| {
            LayoutError::LayoutInvalid(format!("stale flex item id {:?}", item.id))
        })?;
        if node.style().size.width != Dimension::Auto {
            continue;
        }
        let sized = FlexCtx {
            cb: CbSize {
                w: item.main_size,
                h: container_cross,
            },
            ..*ctx
        };
        let edges = NodeEdges::build(node.style(), item.main_size);
        let (cross, _) = item_cross_basis(
            &sized,
            node,
            node.style(),
            ctx.axes,
            item.main_size,
            container_cross,
            &edges,
        );
        if let Some(cross) = cross {
            item.cross_size = clamp(cross, item.min_cross, item.max_cross).max(0.0);
        }
    }
    Ok(())
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

fn line_cross_size(line: &FlexLine) -> f32 {
    line.items
        .iter()
        .map(FlexItem::outer_cross)
        .fold(0.0_f32, f32::max)
}

fn distribute_cross(align: Option<AlignContent>, free: f32, line_count: usize) -> (f32, f32) {
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
    let size =
        if !suppress_stretch && matches!(align, AlignItems::Stretch) && item.cross_size_was_auto {
            (line_cross - item.margin_cross_start() - item.margin_cross_end()).max(0.0)
        } else {
            item.cross_size
        };
    clamp(size, item.min_cross, item.max_cross).max(0.0)
}

fn cross_gap(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> f32 {
    if ctx.axes.row {
        resolve_lp(style.gap.height, ctx.cb.h)
    } else {
        resolve_lp(style.gap.width, ctx.cb.w)
    }
}

fn cross_is_definite(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> bool {
    let (_, size) = main_cross_size(style, ctx.axes);
    let assigned = if ctx.axes.row {
        ctx.assigned_h
    } else {
        ctx.assigned_w
    };
    assigned.is_some() || !matches!(size, Dimension::Auto)
}

fn aligned_line_cross(
    size: f32,
    count: usize,
    definite: bool,
    available: f32,
    free: f32,
    align: Option<AlignContent>,
) -> f32 {
    if !definite {
        return size;
    }
    if count == 1 {
        return available;
    }
    if matches!(align, Some(AlignContent::Stretch)) {
        size + free / count as f32
    } else {
        size
    }
}
