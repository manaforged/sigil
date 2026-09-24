use crate::layout::LayoutError;
use crate::layout::flex::Position2;
use crate::layout::flex_algorithm::{FlexCtx, layout_item_subtree};
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::NodeEdges;
use crate::layout::style_data::{
    AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Position, StyleData,
};

pub(crate) struct Area {
    pub(crate) border_origin: Position2,
    pub(crate) content_origin: Position2,
    pub(crate) main: f32,
    pub(crate) cross: f32,
}

pub(crate) fn pack_definite(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    edges: &NodeEdges,
    area: &Area,
) -> Result<bool, LayoutError> {
    let style = node.style();
    if !container_ok(ctx, style) {
        return Ok(false);
    }
    let forest = ctx.solver.scope().forest();
    let mut cursor = 0.0_f32;
    let mut packed = Vec::with_capacity(node.children().len());
    for child_id in node.children() {
        let child = forest.refer(*child_id).ok_or_else(|| {
            LayoutError::LayoutInvalid(format!("stale flex child id {child_id:?}"))
        })?;
        let child_style = child.style();
        if matches!(child_style.display, Display::None) {
            continue;
        }
        let Some((width, height)) = item_box(child_style) else {
            return Ok(false);
        };
        let main = if ctx.axes.row { width } else { height };
        packed.push((*child_id, width, height, cursor));
        cursor += main;
    }
    if cursor > area.main + 0.01 {
        return Ok(false);
    }
    write_container(ctx, node, edges, area);
    for (id, width, height, main_pos) in packed {
        let (x, y) = if ctx.axes.row {
            (area.content_origin.x + main_pos, area.content_origin.y)
        } else {
            (area.content_origin.x, area.content_origin.y + main_pos)
        };
        ctx.solver.write(
            id,
            ComputedBox {
                x,
                y,
                w: width,
                h: height,
            },
        );
        layout_item_subtree(ctx, id, Position2::new(x, y), width, height)?;
    }
    Ok(true)
}

fn container_ok(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> bool {
    matches!(
        style.flex_direction,
        FlexDirection::Row | FlexDirection::Column
    ) && matches!(style.flex_wrap, FlexWrap::NoWrap)
        && matches!(style.justify_content, None | Some(JustifyContent::Start))
        && matches!(
            style.align_items,
            None | Some(AlignItems::Start | AlignItems::Stretch)
        )
        && gap_zero(style)
        && main_definite(ctx, style)
}

const fn main_definite(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> bool {
    let assigned = if ctx.axes.row {
        ctx.assigned_w
    } else {
        ctx.assigned_h
    };
    if assigned.is_some() {
        return true;
    }
    let main = if ctx.axes.row {
        style.size.width
    } else {
        style.size.height
    };
    !matches!(main, Dimension::Auto)
}

fn gap_zero(style: &StyleData) -> bool {
    zero_lp(style.gap.width) && zero_lp(style.gap.height)
}

fn zero_lp(value: LengthPercentage) -> bool {
    match value {
        LengthPercentage::Px(px) => px == 0.0,
        LengthPercentage::Percent(percent) => percent == 0.0,
        LengthPercentage::Text(length) => length.px() == 0.0,
    }
}

fn item_box(style: &StyleData) -> Option<(f32, f32)> {
    if style.flex_grow > 0.0 || style.order != 0 {
        return None;
    }
    if !matches!(style.flex_basis, Dimension::Auto) {
        return None;
    }
    if matches!(style.position, Position::Absolute) {
        return None;
    }
    if !matches!(
        style.align_self,
        None | Some(AlignSelf::Start | AlignSelf::Stretch)
    ) {
        return None;
    }
    if !margins_zero(style) {
        return None;
    }
    let Dimension::Px(width) = style.size.width else {
        return None;
    };
    let Dimension::Px(height) = style.size.height else {
        return None;
    };
    #[expect(clippy::float_cmp, reason = "clamped returns the input or a bound")]
    if clamped(width, style.min_size.width, style.max_size.width) != width {
        return None;
    }
    #[expect(clippy::float_cmp, reason = "clamped returns the input or a bound")]
    if clamped(height, style.min_size.height, style.max_size.height) != height {
        return None;
    }
    Some((width, height))
}

fn margins_zero(style: &StyleData) -> bool {
    zero_lp_auto(style.margin.left)
        && zero_lp_auto(style.margin.right)
        && zero_lp_auto(style.margin.top)
        && zero_lp_auto(style.margin.bottom)
}

fn zero_lp_auto(value: LengthPercentageAuto) -> bool {
    match value {
        LengthPercentageAuto::Px(px) => px == 0.0,
        LengthPercentageAuto::Percent(percent) => percent == 0.0,
        LengthPercentageAuto::Auto => false,
    }
}

const fn clamped(value: f32, min: Dimension, max: Dimension) -> f32 {
    let after_min = match min {
        Dimension::Px(min) => value.max(min),
        _ => value,
    };
    match max {
        Dimension::Px(max) => after_min.min(max),
        _ => after_min,
    }
}

fn write_container(ctx: &FlexCtx<'_, '_>, node: NodeRef<'_>, edges: &NodeEdges, area: &Area) {
    let (chrome_main, chrome_cross) = if ctx.axes.row {
        (edges.inline_chrome(), edges.block_chrome())
    } else {
        (edges.block_chrome(), edges.inline_chrome())
    };
    let w = ctx.assigned_w.unwrap_or(if ctx.axes.row {
        area.main + chrome_main
    } else {
        area.cross + chrome_cross
    });
    let h = ctx.assigned_h.unwrap_or(if ctx.axes.row {
        area.cross + chrome_cross
    } else {
        area.main + chrome_main
    });
    ctx.solver.write(
        node.id(),
        ComputedBox {
            x: area.border_origin.x,
            y: area.border_origin.y,
            w,
            h,
        },
    );
}
