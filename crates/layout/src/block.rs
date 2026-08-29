#![allow(clippy::redundant_pub_crate)]

use crate::LayoutError;
use crate::block_collapse::OpenMargin;
use crate::block_size::{AxisIntent, SizeIntent};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};
use crate::resolve::NodeEdges;
use crate::style_data::{Display, LengthPercentageAuto, Position, Size, StyleData};

pub(crate) struct BlockSolver<'a> {
    scope: MeasureScope<'a>,
}

impl<'a> BlockSolver<'a> {
    pub(crate) const fn new(scope: MeasureScope<'a>) -> Self {
        Self { scope }
    }

    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError> {
        let avail = self.scope.available();
        let cb = root_containing_block(avail);
        let style = subtree.style();
        let edges = NodeEdges::build(style, cb.width);
        let border_origin = Position2::new(edges.margin.left.unwrap_or(0.0), edges.margin_top());
        let me = solve_box(self, subtree, cb, border_origin)?;
        let _ = me;
        Ok(())
    }

    pub(crate) fn solve_at_assigned(
        &self,
        subtree: NodeRef<'_>,
        cb: Size<f32>,
        origin: Position2,
        assigned_w: Option<f32>,
    ) -> Result<(), LayoutError> {
        let style = subtree.style();
        let edges = NodeEdges::build(style, cb.width);
        let border_origin = Position2::new(
            origin.x + edges.margin.left.unwrap_or(0.0),
            origin.y + edges.margin_top(),
        );
        solve_box_assigned(self, subtree, cb, border_origin, assigned_w).map(|_| ())
    }

    pub(crate) const fn scope(&self) -> &MeasureScope<'_> {
        &self.scope
    }

    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let forest = self.scope.forest();
        if forest.frame_for(id)? != forest.current_frame() {
            return None;
        }
        forest.boxed(id)
    }

    fn write(&self, id: LocalId, cb: ComputedBox) {
        self.scope.write(id, cb);
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Position2 {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Position2 {
    const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug)]
struct Resolved {
    box_h: f32,
    edges: NodeEdges,
}

const fn root_containing_block(avail: crate::forest::AvailableSpace) -> Size<f32> {
    Size {
        width: definite(avail.w),
        height: definite(avail.h),
    }
}

const fn definite(c: crate::forest::SpaceConstraint) -> f32 {
    match c {
        crate::forest::SpaceConstraint::Definite(v) => v,
        crate::forest::SpaceConstraint::MinContent | crate::forest::SpaceConstraint::MaxContent => {
            0.0
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "T6 cutover landed solve_box at 66 lines; structural extraction tracked internally"
)]
fn solve_box(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    cb: Size<f32>,
    border_origin: Position2,
) -> Result<Resolved, LayoutError> {
    solve_box_assigned(solver, node, cb, border_origin, None)
}

fn natural_width(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    intent: &SizeIntent,
    cb_width: f32,
    edges: &NodeEdges,
) -> (f32, Option<Size<f32>>) {
    let text_size = if node.children().is_empty() {
        solver.scope.measure_text(
            node,
            Size {
                width: intent.w.preferred,
                height: intent.h.preferred,
            },
            Some((cb_width - edges.inline_chrome()).max(0.0)),
        )
    } else {
        None
    };
    let width = text_size.map_or_else(
        || width_for(node.style(), intent.w, cb_width, edges),
        |size| size.width + edges.inline_chrome(),
    );
    (width, text_size)
}

fn write_none(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    cb: Size<f32>,
    border_origin: Position2,
) -> Resolved {
    let edges = NodeEdges::build(node.style(), cb.width);
    solver.write(
        node.id(),
        ComputedBox {
            x: border_origin.x,
            y: border_origin.y,
            w: 0.0,
            h: 0.0,
        },
    );
    Resolved { box_h: 0.0, edges }
}

fn solve_box_assigned(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    cb: Size<f32>,
    border_origin: Position2,
    assigned_w: Option<f32>,
) -> Result<Resolved, LayoutError> {
    let style = node.style();
    if matches!(style.display, Display::None) {
        return Ok(write_none(solver, node, cb, border_origin));
    }
    let edges = NodeEdges::build(style, cb.width);
    let intent = SizeIntent::build(style, cb, &edges);
    let (natural_w, text_size) = natural_width(solver, node, &intent, cb.width, &edges);
    let box_w = intent.w.clamp(assigned_w.unwrap_or(natural_w));
    let inner_w = (box_w - edges.inline_chrome()).max(0.0);
    let inner_cb = Size {
        width: inner_w,
        height: intent
            .h
            .preferred
            .map_or(cb.height, |height| height - edges.block_chrome())
            .max(0.0),
    };
    let content_origin = Position2::new(
        border_origin.x + edges.border.left + edges.padding.left,
        border_origin.y + edges.border.top + edges.padding.top,
    );
    let padding_box = Size {
        width: inner_w + edges.padding.left + edges.padding.right,
        height: inner_cb.height + edges.padding.top + edges.padding.bottom,
    };
    let content_height = content_height_of(
        solver,
        node,
        text_size,
        (inner_cb, padding_box),
        content_origin,
    )?;
    let auto_h = content_height + edges.block_chrome();
    let box_h = intent
        .h
        .clamp(intent.h.preferred.unwrap_or(auto_h).max(0.0));
    solver.write(
        node.id(),
        ComputedBox {
            x: border_origin.x,
            y: border_origin.y,
            w: box_w,
            h: box_h,
        },
    );
    Ok(Resolved { box_h, edges })
}

fn content_height_of(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    text_size: Option<Size<f32>>,
    boxes: (Size<f32>, Size<f32>),
    content_origin: Position2,
) -> Result<f32, LayoutError> {
    let (inner_cb, padding_box) = boxes;
    if let Some(size) = text_size {
        return Ok(size.height);
    }
    layout_children(
        solver,
        node,
        inner_cb,
        content_origin,
        padding_box,
        content_origin,
    )
}

fn width_for(style: &StyleData, intent: AxisIntent, cb_w: f32, edges: &NodeEdges) -> f32 {
    if let Some(preferred) = intent.preferred {
        return preferred;
    }
    let lateral = edges.margin.left.unwrap_or(0.0) + edges.margin.right.unwrap_or(0.0);
    let fill = cb_w - lateral;
    let _ = (style.box_sizing, edges);
    fill.max(0.0)
}

#[allow(
    clippy::too_many_arguments,
    reason = "T6 cutover landed layout_children at 6 args; PaddingFrame struct bundle tracked internally"
)]
fn layout_children(
    solver: &BlockSolver<'_>,
    parent: NodeRef<'_>,
    inner_cb: Size<f32>,
    content_origin: Position2,
    padding_box: Size<f32>,
    padding_origin: Position2,
) -> Result<f32, LayoutError> {
    let forest = solver.scope.forest();
    let mut cursor_y = 0.0_f32;
    let mut open = OpenMargin::sealed();
    for child_id in parent.children() {
        let child = forest
            .refer(*child_id)
            .ok_or_else(|| LayoutError::LayoutInvalid(format!("stale child id {child_id:?}")))?;
        let style = child.style();
        if matches!(style.display, Display::None) {
            continue;
        }
        if matches!(style.position, Position::Absolute) {
            place_absolute(solver, child, inner_cb, padding_box, padding_origin)?;
            continue;
        }
        let edges_preview = NodeEdges::build(style, inner_cb.width);
        let gap = open.absorb_top(edges_preview.margin_top());
        cursor_y += gap;
        let child_border_origin = Position2::new(
            content_origin.x + edges_preview.margin.left.unwrap_or(0.0),
            content_origin.y + cursor_y,
        );
        let resolved = match style.display {
            Display::Flex => solve_flex_child(solver, child, inner_cb, child_border_origin)?,
            Display::Grid => solve_grid_child(solver, child, inner_cb, child_border_origin)?,
            Display::Block | Display::None => {
                solve_box(solver, child, inner_cb, child_border_origin)?
            }
        };
        cursor_y += resolved.box_h;
        open.emit_bottom(resolved.edges.margin_bottom());
    }
    Ok(cursor_y)
}

fn solve_flex_child(
    solver: &BlockSolver<'_>,
    child: NodeRef<'_>,
    inner_cb: Size<f32>,
    child_border_origin: Position2,
) -> Result<Resolved, LayoutError> {
    let style = child.style();
    let edges = NodeEdges::build(style, inner_cb.width);
    let scope = solver.scope();
    let nested = crate::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
    let flex = crate::flex::FlexSolver::new(nested);
    let cb = crate::flex::CbSize {
        w: inner_cb.width,
        h: inner_cb.height,
    };
    let pre = crate::flex::Position2::new(
        child_border_origin.x - edges.margin.left.unwrap_or(0.0),
        child_border_origin.y - edges.margin_top(),
    );
    flex.solve_at(child, cb, pre)?;
    let computed = solver.boxed(child.id()).ok_or_else(|| {
        LayoutError::LayoutInvalid(format!(
            "flex subtree did not write a box for {:?}",
            child.id()
        ))
    })?;
    Ok(Resolved {
        box_h: computed.h,
        edges,
    })
}

fn solve_grid_child(
    solver: &BlockSolver<'_>,
    child: NodeRef<'_>,
    inner_cb: Size<f32>,
    child_border_origin: Position2,
) -> Result<Resolved, LayoutError> {
    let style = child.style();
    let edges = NodeEdges::build(style, inner_cb.width);
    let scope = solver.scope();
    let nested = crate::forest::MeasureScope::new(scope.forest(), scope.text(), scope.available());
    let grid = crate::grid::GridSolver::new(nested);
    let border_w = match style.size.width {
        crate::style_data::Dimension::Px(px) => px,
        _ => inner_cb.width,
    };
    let pre = Position2 {
        x: child_border_origin.x - edges.margin.left.unwrap_or(0.0),
        y: child_border_origin.y - edges.margin_top(),
    };
    grid.solve_at(child, border_w, pre)?;
    let computed = solver.boxed(child.id()).ok_or_else(|| {
        LayoutError::LayoutInvalid(format!(
            "grid subtree did not write a box for {:?}",
            child.id()
        ))
    })?;
    Ok(Resolved {
        box_h: computed.h,
        edges,
    })
}

fn place_absolute(
    solver: &BlockSolver<'_>,
    child: NodeRef<'_>,
    inner_cb: Size<f32>,
    padding_box: Size<f32>,
    padding_origin: Position2,
) -> Result<(), LayoutError> {
    let style = child.style();
    let edges = NodeEdges::build(style, inner_cb.width);
    let intent = SizeIntent::build(style, inner_cb, &edges);
    let provisional = Position2::new(
        padding_origin.x + edges.margin.left.unwrap_or(0.0),
        padding_origin.y + edges.margin_top(),
    );
    let (box_w, box_h) = size_absolute(solver, child, &edges, &intent, inner_cb, provisional)?;
    let (x, y) = resolve_absolute_xy(style, &edges, padding_origin, padding_box, (box_w, box_h));
    let dx = x - provisional.x;
    let dy = y - provisional.y;
    if dx != 0.0 || dy != 0.0 {
        shift_subtree(solver, child, dx, dy);
    }
    solver.write(
        child.id(),
        ComputedBox {
            x,
            y,
            w: box_w,
            h: box_h,
        },
    );
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "absolute sizing needs solver + child + resolved edges/intent + inner_cb + provisional; bundling would obscure the data flow"
)]
fn size_absolute(
    solver: &BlockSolver<'_>,
    child: NodeRef<'_>,
    edges: &NodeEdges,
    intent: &SizeIntent,
    inner_cb: Size<f32>,
    provisional: Position2,
) -> Result<(f32, f32), LayoutError> {
    let auto_w =
        (inner_cb.width - edges.margin.left.unwrap_or(0.0) - edges.margin.right.unwrap_or(0.0))
            .max(0.0);
    let box_w = intent.w.clamp(intent.w.preferred.unwrap_or(auto_w));
    let inner_w = (box_w - edges.inline_chrome()).max(0.0);
    let inner_cb_child = Size {
        width: inner_w,
        height: intent.h.preferred.unwrap_or(inner_cb.height),
    };
    let content_origin = Position2::new(
        provisional.x + edges.border.left + edges.padding.left,
        provisional.y + edges.border.top + edges.padding.top,
    );
    let child_padding_box = Size {
        width: inner_w + edges.padding.left + edges.padding.right,
        height: inner_cb_child.height + edges.padding.top + edges.padding.bottom,
    };
    let content_h = layout_children(
        solver,
        child,
        inner_cb_child,
        content_origin,
        child_padding_box,
        content_origin,
    )?;
    let auto_h = content_h + edges.block_chrome();
    let box_h = intent.h.clamp(intent.h.preferred.unwrap_or(auto_h));
    Ok((box_w, box_h))
}

fn resolve_absolute_xy(
    style: &StyleData,
    edges: &NodeEdges,
    padding_origin: Position2,
    padding_box: Size<f32>,
    (box_w, box_h): (f32, f32),
) -> (f32, f32) {
    let x = resolve_inset_axis(InsetAxis {
        leading: style.inset.left,
        trailing: style.inset.right,
        origin: padding_origin.x,
        cb_dim: padding_box.width,
        box_dim: box_w,
        margin_leading: edges.margin.left.unwrap_or(0.0),
        margin_trailing: edges.margin.right.unwrap_or(0.0),
    });
    let y = resolve_inset_axis(InsetAxis {
        leading: style.inset.top,
        trailing: style.inset.bottom,
        origin: padding_origin.y,
        cb_dim: padding_box.height,
        box_dim: box_h,
        margin_leading: edges.margin_top(),
        margin_trailing: edges.margin.bottom.unwrap_or(0.0),
    });
    (x, y)
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

fn shift_subtree(solver: &BlockSolver<'_>, node: NodeRef<'_>, dx: f32, dy: f32) {
    let forest = solver.scope.forest();
    for child_id in node.children() {
        if let Some(child) = forest.refer(*child_id) {
            if let Some(b) = solver.boxed(*child_id) {
                solver.write(
                    *child_id,
                    ComputedBox {
                        x: b.x + dx,
                        y: b.y + dy,
                        w: b.w,
                        h: b.h,
                    },
                );
            }
            shift_subtree(solver, child, dx, dy);
        }
    }
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
