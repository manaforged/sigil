use crate::layout::LayoutError;
use crate::layout::flex::Position2;
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{NodeEdges, dim_to_border_box};
use crate::layout::style_data::{LengthPercentageAuto, StyleData};

use super::intrinsic::{probed_content_extent, text_intrinsic};
use super::{FlexCtx, layout_item_subtree};

pub(super) fn place_absolute(
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
        let measured = text_intrinsic(ctx, child, style);
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
