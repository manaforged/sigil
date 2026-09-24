#![allow(clippy::redundant_pub_crate)]

use crate::layout::flex::CbSize;
use crate::layout::flex_algorithm::{
    Axis2, FlexCtx, align_self_of, automatic_min_main, item_cross_basis, item_main_basis,
    item_min_max_main,
};
use crate::layout::forest::{LocalId, NodeRef};
use crate::layout::resolve::{NodeEdges, clamp, resolve_lp};
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
    pub(crate) margin: ItemMargin,
    pub(crate) align_self: Option<AlignSelf>,
    pub(crate) axes_row: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ItemMargin {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl FlexItem {
    pub(crate) fn build(
        ctx: &FlexCtx<'_, '_>,
        node: NodeRef<'_>,
        cb_main: f32,
        cb_cross: f32,
    ) -> Self {
        let measured = FlexCtx {
            cb: CbSize {
                w: if ctx.axes.row { cb_main } else { cb_cross },
                h: if ctx.axes.row { cb_cross } else { cb_main },
            },
            ..*ctx
        };
        let style = node.style();
        let cb_inline = if ctx.axes.row { cb_main } else { cb_cross };
        let edges = NodeEdges::build(style, cb_inline);
        let basis = item_main_basis(&measured, node, style, ctx.axes, cb_main, &edges, false);
        let (cross_resolved, was_auto) =
            item_cross_basis(&measured, node, style, ctx.axes, cb_main, cb_cross, &edges);
        let (min_main, max_main) = item_min_max_main(style, ctx.axes, cb_main, &edges);
        let cross_axes = Axis2 {
            row: !ctx.axes.row,
            reversed_main: false,
        };
        let (min_cross, max_cross) = item_min_max_main(style, cross_axes, cb_cross, &edges);
        let min_main = min_main.or_else(|| {
            let ellipsizes = node.text().is_some_and(|text| text.ellipsis);
            ((node.children().is_empty() || !ctx.axes.row) && !ellipsizes)
                .then(|| automatic_min_main(style, ctx.axes, basis))
                .flatten()
        });
        let m = edges.margin.or_zero();
        Self {
            id: node.id(),
            order: style.order,
            flex_grow: style.flex_grow,
            flex_shrink: style.flex_shrink,
            flex_basis_resolved: basis,
            main_size: basis,
            cross_size: clamp(cross_resolved.unwrap_or(0.0), min_cross, max_cross).max(0.0),
            cross_size_was_auto: was_auto,
            min_main,
            max_main,
            min_cross,
            max_cross,
            margin: ItemMargin {
                left: m.left,
                right: m.right,
                top: m.top,
                bottom: m.bottom,
            },
            align_self: align_self_of(style),
            axes_row: ctx.axes.row,
        }
    }

    pub(crate) const fn margin_main_start(&self) -> f32 {
        if self.axes_row {
            self.margin.left
        } else {
            self.margin.top
        }
    }

    pub(crate) const fn margin_main_end(&self) -> f32 {
        if self.axes_row {
            self.margin.right
        } else {
            self.margin.bottom
        }
    }

    pub(crate) const fn margin_cross_start(&self) -> f32 {
        if self.axes_row {
            self.margin.top
        } else {
            self.margin.left
        }
    }

    pub(crate) const fn margin_cross_end(&self) -> f32 {
        if self.axes_row {
            self.margin.bottom
        } else {
            self.margin.right
        }
    }

    pub(crate) const fn outer_main(&self) -> f32 {
        self.main_size + self.margin_main_start() + self.margin_main_end()
    }

    pub(crate) const fn outer_main_basis(&self) -> f32 {
        self.flex_basis_resolved + self.margin_main_start() + self.margin_main_end()
    }

    pub(crate) const fn outer_cross(&self) -> f32 {
        self.cross_size + self.margin_cross_start() + self.margin_cross_end()
    }
}

#[derive(Debug, Default)]
pub(crate) struct FlexLine {
    pub(crate) items: Vec<FlexItem>,
    pub(crate) gap: f32,
    pub(crate) cross: f32,
}

impl FlexLine {
    const fn new(gap: f32) -> Self {
        Self {
            items: Vec::new(),
            gap,
            cross: 0.0,
        }
    }
}

pub(crate) fn collect_lines(
    ctx: &FlexCtx<'_, '_>,
    items: Vec<FlexItem>,
    style: &StyleData,
    container_main: f32,
) -> Vec<FlexLine> {
    let gap = main_gap(ctx, style);
    if matches!(style.flex_wrap, FlexWrap::NoWrap) || items.is_empty() {
        let line = FlexLine {
            items,
            gap,
            cross: 0.0,
        };
        return vec![line];
    }
    let mut lines: Vec<FlexLine> = Vec::new();
    let mut current = FlexLine::new(gap);
    let mut used = 0.0_f32;
    for item in items {
        let outer = clamp(item.flex_basis_resolved, item.min_main, item.max_main).max(0.0)
            + item.margin_main_start()
            + item.margin_main_end();
        let add_gap = if current.items.is_empty() { 0.0 } else { gap };
        let next = used + add_gap + outer;
        if !current.items.is_empty() && next > container_main + f32::EPSILON {
            lines.push(std::mem::replace(&mut current, FlexLine::new(gap)));
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

fn main_gap(ctx: &FlexCtx<'_, '_>, style: &StyleData) -> f32 {
    if ctx.axes.row {
        crate::layout::snap::gap(resolve_lp(style.gap.width, ctx.cb.w))
    } else {
        crate::layout::snap::row_gap(resolve_lp(style.gap.height, ctx.cb.h))
    }
}
