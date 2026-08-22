//! Line collection + per-item state for the flex solver.
//!
//! `FlexItem` carries the resolved geometry a single child needs through
//! the algorithm: flex base size, min/max main, cross size + auto flag,
//! grow/shrink factors, margins, alignment override, order. `FlexLine`
//! is the grouping the wrap step produces — a list of items plus a
//! resolved main-axis gap (the container's `gap` projected onto the
//! main axis).

#![allow(clippy::redundant_pub_crate)]

use crate::flex_algorithm::{
    FlexCtx, align_self_of, automatic_min_main, item_cross_basis, item_main_basis,
    item_min_max_main,
};
use crate::forest::{LocalId, NodeRef};
use crate::resolve::{NodeEdges, resolve_lp};
use crate::style_data::{AlignSelf, FlexWrap, StyleData};

/// Resolved state for a single flex item.
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
    pub(crate) margin: ItemMargin,
    pub(crate) align_self: Option<AlignSelf>,
    pub(crate) axes_row: bool,
}

/// Per-item margins in (left/right/top/bottom) pixel space.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ItemMargin {
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

impl FlexItem {
    /// Build a flex item from a node ref + style snapshot.
    pub(crate) fn build(
        ctx: &FlexCtx<'_, '_>,
        node: NodeRef<'_>,
        cb_main: f32,
        cb_cross: f32,
    ) -> Self {
        let style = node.style();
        let cb_inline = if ctx.axes.row { cb_main } else { cb_cross };
        let edges = NodeEdges::build(style, cb_inline);
        let basis = item_main_basis(ctx, node, style, ctx.axes, cb_main, &edges);
        let (cross_resolved, was_auto) =
            item_cross_basis(ctx, node, style, ctx.axes, cb_main, cb_cross, &edges);
        let (min_main, max_main) = item_min_max_main(style, ctx.axes, cb_main, &edges);
        // CSS §4.5 automatic minimum size, scoped to leaf items (text and
        // other childless content). `basis` is their content main size.
        // A crushed text leaf is what collapses stacked labels on top of
        // each other; flooring leaves fixes that. Containers are left free
        // to shrink so an over-full parent overflows/scrolls rather than
        // pushing siblings off-screen — matching the existing engine
        // behaviour the golden corpus was baked against.
        let min_main = min_main.or_else(|| {
            // An ellipsizing leaf truncates to whatever main size it is
            // given, so its automatic minimum is zero — flooring it at
            // its content width would crush its fixed-width siblings in
            // an over-full line instead.
            let ellipsizes = node.text().is_some_and(|text| text.ellipsis);
            (node.children().is_empty() && !ellipsizes)
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
            cross_size: cross_resolved.unwrap_or(0.0),
            cross_size_was_auto: was_auto,
            min_main,
            max_main,
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

    /// Outer main extent at the current (post-resolve) `main_size`.
    pub(crate) const fn outer_main(&self) -> f32 {
        self.main_size + self.margin_main_start() + self.margin_main_end()
    }

    /// Outer main extent using the pre-resolve basis (used by wrap +
    /// flexible-length space accounting).
    pub(crate) const fn outer_main_basis(&self) -> f32 {
        self.flex_basis_resolved + self.margin_main_start() + self.margin_main_end()
    }

    /// Outer cross extent.
    pub(crate) const fn outer_cross(&self) -> f32 {
        self.cross_size + self.margin_cross_start() + self.margin_cross_end()
    }
}

/// One flex line — list of items + resolved main-axis gap + tallest
/// outer cross (filled in by the placement step).
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

/// Collect items into one or more lines per `FlexWrap`.
///
/// Single line when wrap is `NoWrap`. Otherwise greedy fill —
/// accumulate outer-main of items until the next would overflow
/// `container_main`; start a new line.
///
/// Takes `items` by value so the `NoWrap` path can move the whole vec
/// into a single `FlexLine` without copying. The wide-tree corpus
/// (one container, 100k+ flex children) makes this copy a load-bearing
/// cost — `extend_from_slice` previously moved ~8 MiB of `FlexItem`
/// structs on every call.
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
        let outer = item.outer_main_basis();
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
        resolve_lp(style.gap.width, ctx.cb.w)
    } else {
        resolve_lp(style.gap.height, ctx.cb.h)
    }
}
