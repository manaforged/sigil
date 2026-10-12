use crate::layout::LayoutError;
use crate::layout::flex::{CbSize, Position2};
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
        let Some((width, height)) = child_style.item_box() else {
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
        && style.gap_zero()
        && ctx.definite_along(style, ctx.axes.row)
}

fn zero_lp(value: LengthPercentage) -> bool {
    match value {
        LengthPercentage::Px(px) => px == 0.0,
        LengthPercentage::Percent(percent) => percent == 0.0,
        LengthPercentage::Text(length) => length.px() == 0.0,
    }
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
    let (chrome_main, chrome_cross) = ctx.axes.chrome(edges);
    let size = CbSize::across(ctx.axes, area.main + chrome_main, area.cross + chrome_cross);
    ctx.solver.write(
        node.id(),
        ComputedBox {
            x: area.border_origin.x,
            y: area.border_origin.y,
            w: ctx.assigned_w.unwrap_or(size.w),
            h: ctx.assigned_h.unwrap_or(size.h),
        },
    );
}

impl StyleData {
    fn gap_zero(&self) -> bool {
        zero_lp(self.gap.width) && zero_lp(self.gap.height)
    }

    fn item_box(&self) -> Option<(f32, f32)> {
        let plain = self.flex_grow <= 0.0
            && self.order == 0
            && matches!(self.flex_basis, Dimension::Auto)
            && !matches!(self.position, Position::Absolute)
            && matches!(
                self.align_self,
                None | Some(AlignSelf::Start | AlignSelf::Stretch)
            )
            && self.margins_zero();
        let (Dimension::Px(width), Dimension::Px(height)) = (self.size.width, self.size.height)
        else {
            return None;
        };
        #[expect(clippy::float_cmp, reason = "clamped returns the input or a bound")]
        let unclamped = clamped(width, self.min_size.width, self.max_size.width) == width
            && clamped(height, self.min_size.height, self.max_size.height) == height;
        (plain && unclamped).then_some((width, height))
    }

    fn margins_zero(&self) -> bool {
        zero_lp_auto(self.margin.left)
            && zero_lp_auto(self.margin.right)
            && zero_lp_auto(self.margin.top)
            && zero_lp_auto(self.margin.bottom)
    }
}
