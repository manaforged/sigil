use super::{BlockSolver, Position2, layout_children, solve_flex_child, solve_grid_child};
use crate::layout::LayoutError;
use crate::layout::block_size::SizeIntent;
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{InsetAxis, NodeEdges, resolve_inset_axis};
use crate::layout::style_data::{Display, Size, StyleData};

pub(super) fn place_absolute(
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
    match child.style().display {
        Display::Flex => {
            solve_flex_child(solver, child, inner_cb, provisional)?;
            return computed_size(solver, child);
        }
        Display::Grid => {
            solve_grid_child(solver, child, inner_cb, provisional)?;
            return computed_size(solver, child);
        }
        Display::Block | Display::None => {}
    }
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

fn computed_size(solver: &BlockSolver<'_>, child: NodeRef<'_>) -> Result<(f32, f32), LayoutError> {
    solver
        .boxed(child.id())
        .map(|bounds| (bounds.w, bounds.h))
        .ok_or_else(|| {
            LayoutError::LayoutInvalid(format!(
                "absolute subtree did not write a box for {:?}",
                child.id()
            ))
        })
}
