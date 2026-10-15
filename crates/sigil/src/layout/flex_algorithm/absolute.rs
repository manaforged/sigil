use crate::layout::LayoutError;
use crate::layout::flex::{CbSize, Position2};
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{
    InsetAxis, NodeEdges, dim_to_border_box, resolve_inset_axis, stretched,
};
use crate::layout::style_data::{Display, LengthPercentageAuto, Position, Size, StyleData};

use super::intrinsic::{probed_content_extent, text_intrinsic};
use super::{FlexCtx, layout_item_subtree};

pub(super) fn place_absolutes(
    ctx: &FlexCtx<'_, '_>,
    node: NodeRef<'_>,
    edges: &NodeEdges,
    border_origin: Position2,
) -> Result<(), LayoutError> {
    let forest = ctx.solver.scope().forest();
    let Some(frame) = forest.boxed(node.id()) else {
        return Ok(());
    };
    let padding_box = Size {
        width: (frame.w - edges.border.left - edges.border.right).max(0.0),
        height: (frame.h - edges.border.top - edges.border.bottom).max(0.0),
    };
    let content = CbSize {
        w: (padding_box.width - edges.padding.left - edges.padding.right).max(0.0),
        h: (padding_box.height - edges.padding.top - edges.padding.bottom).max(0.0),
    };
    let (cb_main, cb_cross) = if ctx.axes.row {
        (content.w, content.h)
    } else {
        (content.h, content.w)
    };
    for child_id in node.children() {
        let Some(child) = forest.refer(*child_id) else {
            continue;
        };
        let style = child.style();
        if matches!(style.position, Position::Absolute) && !matches!(style.display, Display::None) {
            place_absolute(ctx, child, (cb_main, cb_cross), padding_box, border_origin)?;
        }
    }
    Ok(())
}

fn place_absolute(
    ctx: &FlexCtx<'_, '_>,
    child: NodeRef<'_>,
    (cb_main, cb_cross): (f32, f32),
    padding_box: Size<f32>,
    padding_origin: Position2,
) -> Result<(), LayoutError> {
    let style = child.style();
    let CbSize { w: cb_w, h: cb_h } = CbSize::across(ctx.axes, cb_main, cb_cross);
    let edges = NodeEdges::build(style, cb_w);
    let margins = edges.margin.or_zero();
    let (w, h) = absolute_size(ctx, child, style, (cb_w, cb_h), padding_box);
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
    (cb_w, cb_h): (f32, f32),
    padding_box: Size<f32>,
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
    let explicit_w = explicit_w.or_else(|| {
        stretched(
            [style.inset.left, style.inset.right],
            padding_box.width,
            margins.left + margins.right,
        )
    });
    let explicit_h = explicit_h.or_else(|| {
        stretched(
            [style.inset.top, style.inset.bottom],
            padding_box.height,
            margins.top + margins.bottom,
        )
    });
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
        let measured = text_intrinsic(ctx, child, style);
        return Some((
            measured.map_or(0.0, |m| m.width) + edges.inline_chrome(),
            measured.map_or(0.0, |m| m.height) + edges.block_chrome(),
        ));
    }
    layout_item_subtree(
        &ctx.probe(),
        child.id(),
        Position2::new(0.0, 0.0),
        cb_w,
        cb_h,
    )
    .ok()?;
    Some((
        probed_content_extent(ctx, child, ctx.axes.row, ctx.axes),
        probed_content_extent(ctx, child, !ctx.axes.row, ctx.axes),
    ))
}
