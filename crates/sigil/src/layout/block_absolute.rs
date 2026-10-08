use super::{BlockSolver, Position2};
use crate::layout::LayoutError;
use crate::layout::block_size::SizeIntent;
use crate::layout::forest::{ComputedBox, NodeRef};
use crate::layout::resolve::{InsetAxis, NodeEdges, resolve_inset_axis};
use crate::layout::style_data::{Display, Size, StyleData};

impl BlockSolver<'_> {
    pub(super) fn place_absolute(
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
        let (box_w, box_h) = self.size_absolute(child, &edges, &intent, inner_cb, provisional)?;
        let (x, y) =
            resolve_absolute_xy(style, &edges, padding_origin, padding_box, (box_w, box_h));
        let dx = x - provisional.x;
        let dy = y - provisional.y;
        if dx != 0.0 || dy != 0.0 {
            self.shift_subtree(child, dx, dy);
        }
        self.write(
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
        reason = "absolute sizing needs self + child + resolved edges/intent + inner_cb + provisional; bundling would obscure the data flow"
    )]
    fn size_absolute(
        &self,
        child: NodeRef<'_>,
        edges: &NodeEdges,
        intent: &SizeIntent,
        inner_cb: Size<f32>,
        provisional: Position2,
    ) -> Result<(f32, f32), LayoutError> {
        match child.style().display {
            Display::Flex => {
                self.solve_flex_child(child, inner_cb, provisional)?;
                return self.computed_size(child);
            }
            Display::Grid => {
                self.solve_grid_child(child, inner_cb, provisional)?;
                return self.computed_size(child);
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
        let content_h = self.layout_children(
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

    fn shift_subtree(&self, node: NodeRef<'_>, dx: f32, dy: f32) {
        let forest = self.scope.forest();
        for child_id in node.children() {
            if let Some(child) = forest.refer(*child_id) {
                if let Some(b) = self.boxed(*child_id) {
                    self.write(
                        *child_id,
                        ComputedBox {
                            x: b.x + dx,
                            y: b.y + dy,
                            w: b.w,
                            h: b.h,
                        },
                    );
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
