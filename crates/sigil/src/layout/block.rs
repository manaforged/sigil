#![allow(clippy::redundant_pub_crate)]

use crate::layout::LayoutError;
use crate::layout::block_collapse::OpenMargin;
use crate::layout::block_size::SizeIntent;
use crate::layout::flex::{CbSize, FlexSolver};
use crate::layout::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};
use crate::layout::grid::GridSolver;
use crate::layout::grid_tracks::definite;
use crate::layout::resolve::NodeEdges;
use crate::layout::style_data::{Dimension, Display, Position, Size, StyleData};

#[path = "block_absolute.rs"]
mod absolute;

pub(crate) struct BlockSolver<'a> {
    scope: MeasureScope<'a>,
}

impl<'a> BlockSolver<'a> {
    pub(crate) const fn new(scope: MeasureScope<'a>) -> Self {
        Self { scope }
    }

    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError> {
        let avail = self.scope.available();
        let cb = Size {
            width: definite(avail.w),
            height: definite(avail.h),
        };
        self.solve_at_assigned(subtree, cb, Position2::new(0.0, 0.0), Size::default())
    }

    pub(crate) fn solve_at_assigned(
        &self,
        subtree: NodeRef<'_>,
        cb: Size<f32>,
        origin: Position2,
        assigned: Size<Option<f32>>,
    ) -> Result<(), LayoutError> {
        let edges = NodeEdges::build(subtree.style(), cb.width);
        let border_origin = Position2::new(
            origin.x + edges.margin.left.unwrap_or(0.0),
            origin.y + edges.margin_top(),
        );
        self.solve_box(subtree, cb, border_origin, assigned)
            .map(|_| ())
    }

    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let forest = self.scope.forest();
        if forest.frame_for(id)? != forest.current_frame() {
            return None;
        }
        forest.boxed(id)
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

fn child_border_x(
    parent: &StyleData,
    style: &StyleData,
    inner_cb: Size<f32>,
    content_x: f32,
    edges: &NodeEdges,
) -> f32 {
    let start = content_x + edges.margin.left.unwrap_or(0.0);
    if !parent.direction.is_rtl() {
        return start;
    }
    let intent = SizeIntent::build(style, inner_cb, edges);
    intent.w.preferred.map_or(start, |width| {
        content_x + inner_cb.width - edges.margin.right.unwrap_or(0.0) - intent.w.clamp(width)
    })
}

impl BlockSolver<'_> {
    fn natural_width(
        &self,
        node: NodeRef<'_>,
        intent: &SizeIntent,
        cb_width: f32,
        edges: &NodeEdges,
    ) -> (f32, Option<Size<f32>>) {
        let text_size = if node.children().is_empty() {
            let preferred = Size {
                width: intent.w.preferred,
                height: intent.h.preferred,
            };
            self.scope
                .measure_leaf_text(node, preferred, cb_width, edges)
        } else {
            None
        };
        let width = text_size.map_or_else(
            || {
                let lateral = edges.margin.left.unwrap_or(0.0) + edges.margin.right.unwrap_or(0.0);
                intent
                    .w
                    .preferred
                    .unwrap_or_else(|| (cb_width - lateral).max(0.0))
            },
            |size| size.width + edges.inline_chrome(),
        );
        (width, text_size)
    }

    fn solve_box(
        &self,
        node: NodeRef<'_>,
        cb: Size<f32>,
        border_origin: Position2,
        assigned: Size<Option<f32>>,
    ) -> Result<Resolved, LayoutError> {
        let style = node.style();
        let edges = NodeEdges::build(style, cb.width);
        let (x, y) = (border_origin.x, border_origin.y);
        if matches!(style.display, Display::None) {
            let empty = ComputedBox {
                x,
                y,
                ..ComputedBox::default()
            };
            self.scope.write(node.id(), empty);
            return Ok(Resolved { box_h: 0.0, edges });
        }
        let intent = SizeIntent::build(style, cb, &edges);
        let (natural_w, text_size) = self.natural_width(node, &intent, cb.width, &edges);
        let w = intent.w.clamp(assigned.width.unwrap_or(natural_w));
        let inner_cb = Size {
            width: (w - edges.inline_chrome()).max(0.0),
            height: intent
                .h
                .preferred
                .map_or(cb.height, |height| height - edges.block_chrome())
                .max(0.0),
        };
        let content_height = match text_size {
            Some(size) => size.height,
            None => self.layout_children(node, inner_cb, border_origin, &edges)?,
        };
        let auto_h = content_height + edges.block_chrome();
        let h = assigned.height.unwrap_or_else(|| {
            intent
                .h
                .clamp(intent.h.preferred.unwrap_or(auto_h).max(0.0))
        });
        self.scope.write(node.id(), ComputedBox { x, y, w, h });
        if text_size.is_none() {
            self.place_absolutes(node, (w, h), border_origin, &edges)?;
        }
        Ok(Resolved { box_h: h, edges })
    }

    fn layout_children(
        &self,
        parent: NodeRef<'_>,
        inner_cb: Size<f32>,
        border_origin: Position2,
        edges: &NodeEdges,
    ) -> Result<f32, LayoutError> {
        let origin = Position2::new(
            border_origin.x + edges.border.left + edges.padding.left,
            border_origin.y + edges.border.top + edges.padding.top,
        );
        let mut cursor_y = 0.0_f32;
        let mut open = OpenMargin::sealed();
        for child_id in parent.children() {
            let child = self.scope.forest().refer(*child_id).ok_or_else(|| {
                LayoutError::LayoutInvalid(format!("stale child id {child_id:?}"))
            })?;
            let style = child.style();
            if matches!(style.display, Display::None) {
                continue;
            }
            if matches!(style.position, Position::Absolute) {
                continue;
            }
            let child_edges = NodeEdges::build(style, inner_cb.width);
            cursor_y += open.absorb_top(child_edges.margin_top());
            let x = child_border_x(parent.style(), style, inner_cb, origin.x, &child_edges);
            let child_origin = Position2::new(x, origin.y + cursor_y);
            let resolved = match style.display {
                Display::Flex | Display::Grid => {
                    self.solve_nested(child, inner_cb, child_origin)?
                }
                Display::Block | Display::None => {
                    self.solve_box(child, inner_cb, child_origin, Size::default())?
                }
            };
            cursor_y += resolved.box_h;
            open.emit_bottom(resolved.edges.margin_bottom());
        }
        Ok(cursor_y)
    }

    fn solve_nested(
        &self,
        child: NodeRef<'_>,
        inner_cb: Size<f32>,
        border_origin: Position2,
    ) -> Result<Resolved, LayoutError> {
        let style = child.style();
        let edges = NodeEdges::build(style, inner_cb.width);
        let scope = &self.scope;
        let nested = MeasureScope::new(scope.forest(), scope.text(), scope.available());
        let x = border_origin.x - edges.margin.left.unwrap_or(0.0);
        let y = border_origin.y - edges.margin_top();
        let kind = if matches!(style.display, Display::Grid) {
            let border_w = match style.size.width {
                Dimension::Px(px) => px,
                _ => inner_cb.width,
            };
            GridSolver::new(nested).solve_at(child, border_w, Position2 { x, y })?;
            "grid"
        } else {
            let cb = CbSize {
                w: inner_cb.width,
                h: inner_cb.height,
            };
            let origin = crate::layout::flex::Position2::new(x, y);
            FlexSolver::new(nested).solve_at(child, cb, origin)?;
            "flex"
        };
        let computed = self.boxed(child.id()).ok_or_else(|| {
            LayoutError::LayoutInvalid(format!(
                "{kind} subtree did not write a box for {:?}",
                child.id()
            ))
        })?;
        Ok(Resolved {
            box_h: computed.h,
            edges,
        })
    }
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
