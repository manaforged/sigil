use super::{BlockSolver, Position2};
use crate::layout::LayoutError;
use crate::layout::block_size::SizeIntent;
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{InsetAxis, NodeEdges, resolve_inset_axis, stretched};
use crate::layout::style_data::{Display, Position, Size, StyleData};

impl BlockSolver<'_> {
    pub(super) fn place_absolutes(
        &self,
        node: NodeRef<'_>,
        (w, h): (f32, f32),
        border_origin: Position2,
        edges: &NodeEdges,
    ) -> Result<(), LayoutError> {
        let padding_box = Size {
            width: (w - edges.border.left - edges.border.right).max(0.0),
            height: (h - edges.border.top - edges.border.bottom).max(0.0),
        };
        let inner_cb = Size {
            width: (w - edges.inline_chrome()).max(0.0),
            height: (h - edges.block_chrome()).max(0.0),
        };
        let origin = Position2::new(
            border_origin.x + edges.border.left + edges.padding.left,
            border_origin.y + edges.border.top + edges.padding.top,
        );
        let forest = self.scope.forest();
        for child_id in node.children() {
            let Some(child) = forest.refer(*child_id) else {
                continue;
            };
            let style = child.style();
            if matches!(style.position, Position::Absolute)
                && !matches!(style.display, Display::None)
            {
                self.place_absolute(child, inner_cb, padding_box, origin)?;
            }
        }
        Ok(())
    }

    fn place_absolute(
        &self,
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
        let (box_w, box_h) =
            self.size_absolute(child, &edges, &intent, (inner_cb, padding_box), provisional)?;
        let (x, y) =
            resolve_absolute_xy(style, &edges, padding_origin, padding_box, (box_w, box_h));
        let dx = x - provisional.x;
        let dy = y - provisional.y;
        if dx != 0.0 || dy != 0.0 {
            self.shift_subtree(child, dx, dy);
        }
        let (w, h) = (box_w, box_h);
        self.scope.write(child.id(), ComputedBox { x, y, w, h });
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one absolute child needs its box, edges, intent and containing block"
    )]
    fn size_absolute(
        &self,
        child: NodeRef<'_>,
        edges: &NodeEdges,
        intent: &SizeIntent,
        (inner_cb, padding_box): (Size<f32>, Size<f32>),
        provisional: Position2,
    ) -> Result<(f32, f32), LayoutError> {
        let style = child.style();
        let margins = edges.margin.left.unwrap_or(0.0) + edges.margin.right.unwrap_or(0.0);
        let stretch_w = stretched(
            [style.inset.left, style.inset.right],
            padding_box.width,
            margins,
        );
        let stretch_h = stretched(
            [style.inset.top, style.inset.bottom],
            padding_box.height,
            edges.margin_top() + edges.margin.bottom.unwrap_or(0.0),
        );
        if matches!(child.style().display, Display::Flex | Display::Grid) {
            self.solve_nested(child, inner_cb, provisional)?;
            return self.computed_size(child);
        }
        let auto_w =
            (inner_cb.width - edges.margin.left.unwrap_or(0.0) - edges.margin.right.unwrap_or(0.0))
                .max(0.0);
        let box_w = intent
            .w
            .clamp(intent.w.preferred.or(stretch_w).unwrap_or(auto_w));
        let inner_w = (box_w - edges.inline_chrome()).max(0.0);
        let inner_cb_child = Size {
            width: inner_w,
            height: intent.h.preferred.or(stretch_h).unwrap_or(inner_cb.height),
        };
        let content_h = self.layout_children(child, inner_cb_child, provisional, edges)?;
        let auto_h = content_h + edges.block_chrome();
        let box_h = intent
            .h
            .clamp(intent.h.preferred.or(stretch_h).unwrap_or(auto_h));
        self.place_absolutes(child, (box_w, box_h), provisional, edges)?;
        Ok((box_w, box_h))
    }

    fn shift_subtree(&self, node: NodeRef<'_>, dx: f32, dy: f32) {
        let forest = self.scope.forest();
        for child_id in node.children() {
            if let Some(child) = forest.refer(*child_id) {
                if let Some(b) = self.boxed(*child_id) {
                    let (x, y) = (b.x + dx, b.y + dy);
                    self.scope.write(*child_id, ComputedBox { x, y, ..b });
                }
                self.shift_subtree(child, dx, dy);
            }
        }
    }

    fn computed_size(&self, child: NodeRef<'_>) -> Result<(f32, f32), LayoutError> {
        self.boxed(child.id())
            .map(|bounds| (bounds.w, bounds.h))
            .ok_or_else(|| {
                LayoutError::LayoutInvalid(format!(
                    "absolute subtree did not write a box for {:?}",
                    child.id()
                ))
            })
    }
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
