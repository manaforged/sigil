use super::{BlockSolver, Position2};
use crate::Error;
use crate::layout::block_size::SizeIntent;
use crate::layout::flex::{CbSize, FlexSolver};
use crate::layout::forest::{ComputedBox, MeasureScope, NodeRef};
use crate::layout::resolve::{NodeEdges, resolve_absolute_xy, stretched};
use crate::layout::style_data::{Axes, Display, Position};

impl BlockSolver<'_> {
    pub(super) fn place_absolutes(
        &self,
        node: NodeRef<'_>,
        (w, h): (f32, f32),
        border_origin: Position2,
        edges: &NodeEdges,
    ) -> Result<(), Error> {
        let padding_box = Axes {
            width: (w - edges.border.left - edges.border.right).max(0.0),
            height: (h - edges.border.top - edges.border.bottom).max(0.0),
        };
        let inner_cb = Axes {
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
        inner_cb: Axes<f32>,
        padding_box: Axes<f32>,
        padding_origin: Position2,
    ) -> Result<(), Error> {
        let style = child.style();
        let edges = child.edges(inner_cb.width);
        let intent = SizeIntent::build(style, inner_cb, &edges);
        let provisional = Position2::new(
            padding_origin.x + edges.margin.left.unwrap_or(0.0),
            padding_origin.y + edges.margin_top(),
        );
        let (box_w, box_h) = self.size_absolute(child, &edges, &intent, inner_cb, provisional)?;
        let (x, y) = resolve_absolute_xy(
            style,
            &edges,
            (padding_origin.x, padding_origin.y),
            padding_box,
            (box_w, box_h),
        );
        let dx = x - provisional.x;
        let dy = y - provisional.y;
        if dx != 0.0 || dy != 0.0 {
            self.scope.forest().forget_solved_subtree(child.id());
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
        inner_cb: Axes<f32>,
        provisional: Position2,
    ) -> Result<(f32, f32), Error> {
        let style = child.style();
        let margins = edges.margin.left.unwrap_or(0.0) + edges.margin.right.unwrap_or(0.0);
        let stretch_w = stretched(
            [style.inset.left, style.inset.right],
            inner_cb.width,
            margins,
        );
        let stretch_h = stretched(
            [style.inset.top, style.inset.bottom],
            inner_cb.height,
            edges.margin_top() + edges.margin.bottom.unwrap_or(0.0),
        );
        let display = child.style().display;
        if matches!(display, Display::Flex) && (stretch_w.is_some() || stretch_h.is_some()) {
            let area = CbSize {
                w: stretch_w.unwrap_or(inner_cb.width),
                h: stretch_h.unwrap_or(inner_cb.height),
            };
            let origin = Position2::new(
                provisional.x - edges.margin.left.unwrap_or(0.0),
                provisional.y - edges.margin_top(),
            );
            let scope = &self.scope;
            let nested = MeasureScope::new(scope.forest(), scope.text(), scope.available());
            FlexSolver::new(nested).solve_at_area(child, area, origin)?;
            return self.computed_size(child);
        }
        if matches!(display, Display::Flex | Display::Grid) {
            self.solve_nested(child, inner_cb, provisional, *edges)?;
            return self.computed_size(child);
        }
        let auto_w =
            (inner_cb.width - edges.margin.left.unwrap_or(0.0) - edges.margin.right.unwrap_or(0.0))
                .max(0.0);
        let box_w = intent
            .w
            .clamp(intent.w.preferred.or(stretch_w).unwrap_or(auto_w));
        let inner_w = (box_w - edges.inline_chrome()).max(0.0);
        let inner_cb_child = Axes {
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

    fn computed_size(&self, child: NodeRef<'_>) -> Result<(f32, f32), Error> {
        self.boxed(child.id())
            .map(|bounds| (bounds.w, bounds.h))
            .ok_or_else(|| {
                Error::layout(format!(
                    "absolute subtree did not write a box for {:?}",
                    child.id()
                ))
            })
    }
}
