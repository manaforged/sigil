#![allow(clippy::redundant_pub_crate)]

use crate::layout::flex::CbSize;
use crate::layout::flex_algorithm::{
    FlexCtx, automatic_min_main, gap_along, item_cross_basis, item_main_basis, item_min_max_main,
};
use crate::layout::forest::{LocalId, NodeRef};
use crate::layout::resolve::{NodeEdges, clamp};
use crate::layout::style_data::{AlignSelf, FlexWrap, StyleData};

#[derive(Clone, Copy, Debug)]
pub(crate) struct FlexItem {
    pub(crate) id: LocalId,
    pub(crate) order: i32,
    pub(crate) flex_grow: f32,
    pub(crate) flex_shrink: f32,
    pub(crate) flex_basis_resolved: f32,
    pub(crate) main_size: f32,
    pub(crate) cross_size: f32,
    pub(crate) cross_size_was_auto: bool,
    pub(crate) min_main: Option<f32>,
    pub(crate) max_main: Option<f32>,
    pub(crate) min_cross: Option<f32>,
    pub(crate) max_cross: Option<f32>,
    pub(crate) margin_main_start: f32,
    pub(crate) margin_main_end: f32,
    pub(crate) margin_cross_start: f32,
    pub(crate) margin_cross_end: f32,
    pub(crate) align_self: Option<AlignSelf>,
}

impl FlexItem {
    pub(crate) fn build(
        ctx: &FlexCtx<'_, '_>,
        node: NodeRef<'_>,
        cb_main: f32,
        cb_cross: f32,
    ) -> Self {
        let measured = FlexCtx {
            cb: CbSize::across(ctx.axes, cb_main, cb_cross),
            ..*ctx
        };
        let style = node.style();
        let edges = NodeEdges::build(style, measured.cb.w);
        let basis = item_main_basis(&measured, node, style, ctx.axes, cb_main, &edges, false);
        let (cross_resolved, was_auto) =
            item_cross_basis(&measured, node, style, ctx.axes, cb_main, cb_cross, &edges);
        let (min_main, max_main) = item_min_max_main(style, ctx.axes, cb_main, &edges);
        let (min_cross, max_cross) = item_min_max_main(style, ctx.axes.cross(), cb_cross, &edges);
        let min_main = min_main.or_else(|| {
            let ellipsizes = node.text().is_some_and(|text| text.ellipsis);
            ((node.children().is_empty() || !ctx.axes.row) && !ellipsizes)
                .then(|| automatic_min_main(style, ctx.axes, basis))
                .flatten()
        });
        let m = edges.margin.or_zero();
        let fluid = ctx.axes.row && style.flex_grow <= 0.0 && ctx.solver.fills_width(node);
        Self {
            id: node.id(),
            order: style.order,
            flex_grow: if fluid { 1.0 } else { style.flex_grow },
            flex_shrink: style.flex_shrink,
            flex_basis_resolved: basis,
            main_size: basis,
            cross_size: clamp(cross_resolved.unwrap_or(0.0), min_cross, max_cross).max(0.0),
            cross_size_was_auto: was_auto,
            min_main,
            max_main,
            min_cross,
            max_cross,
            margin_main_start: ctx.axes.pick(m.left, m.top),
            margin_main_end: ctx.axes.pick(m.right, m.bottom),
            margin_cross_start: ctx.axes.pick(m.top, m.left),
            margin_cross_end: ctx.axes.pick(m.bottom, m.right),
            align_self: style.align_self,
        }
    }

    pub(crate) const fn outer_main(&self) -> f32 {
        self.main_size + self.margin_main_start + self.margin_main_end
    }

    pub(crate) const fn outer_main_basis(&self) -> f32 {
        self.flex_basis_resolved + self.margin_main_start + self.margin_main_end
    }

    pub(crate) const fn outer_cross(&self) -> f32 {
        self.cross_size + self.margin_cross_start + self.margin_cross_end
    }
}

#[derive(Debug, Default)]
pub(crate) struct FlexLine {
    pub(crate) items: Vec<FlexItem>,
    pub(crate) gap: f32,
    pub(crate) cross: f32,
}

pub(crate) fn collect_lines(
    ctx: &FlexCtx<'_, '_>,
    items: Vec<FlexItem>,
    style: &StyleData,
    container_main: f32,
) -> Vec<FlexLine> {
    let gap = gap_along(ctx, style, ctx.axes.row);
    let line = |members| FlexLine {
        items: members,
        gap,
        cross: 0.0,
    };
    if matches!(style.flex_wrap, FlexWrap::NoWrap) || items.is_empty() {
        return vec![line(items)];
    }
    let mut lines = Vec::new();
    let mut current = line(Vec::new());
    let mut used = 0.0_f32;
    for item in items {
        let outer = clamp(item.flex_basis_resolved, item.min_main, item.max_main).max(0.0)
            + item.margin_main_start
            + item.margin_main_end;
        let add_gap = if current.items.is_empty() { 0.0 } else { gap };
        let next = used + add_gap + outer;
        if !current.items.is_empty() && next > container_main + f32::EPSILON {
            lines.push(std::mem::replace(&mut current, line(Vec::new())));
            used = outer;
        } else {
            used = next;
        }
        current.items.push(item);
    }
    if !current.items.is_empty() {
        lines.push(current);
    }
    lines
}
