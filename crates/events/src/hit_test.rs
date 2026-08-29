use sigil_hal::{Point, Rect};
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;
use smallvec::SmallVec;

pub(crate) type NodePath = SmallVec<[NodeId; 64]>;

type HitItems = SmallVec<[HitItem; 64]>;

#[derive(Debug)]
pub(crate) struct HitTestCache {
    cx_generation: u64,
    layout_generation: u64,
    items: HitItems,
}

#[derive(Clone, Copy, Debug)]
struct HitItem {
    id: NodeId,
    rect: Rect,
    has_rect: bool,
    hittable: bool,
}

impl Default for HitTestCache {
    fn default() -> Self {
        Self {
            cx_generation: u64::MAX,
            layout_generation: u64::MAX,
            items: HitItems::new(),
        }
    }
}

impl HitTestCache {
    pub(crate) fn hit_path(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        position: Point,
    ) -> Option<NodePath> {
        self.refresh(cx, layout);
        let hit = self.hit(position)?;
        Some(build_path(cx, hit))
    }

    fn refresh(&mut self, cx: &Cx, layout: &LayoutEngine) {
        let cx_generation = cx.mutation_generation();
        let layout_generation = layout.generation();
        if self.cx_generation == cx_generation && self.layout_generation == layout_generation {
            return;
        }
        self.items.clear();
        let ctx = EmitCtx { cx, layout };
        for root in cx.roots().iter().copied() {
            emit(&ctx, root, &mut self.items);
        }
        self.cx_generation = cx_generation;
        self.layout_generation = layout_generation;
    }

    fn hit(&self, position: Point) -> Option<NodeId> {
        for item in self.items.iter().rev() {
            if item.has_rect && item.hittable && contains(item.rect, position) {
                return Some(item.id);
            }
        }
        None
    }
}

fn build_path(cx: &Cx, target: NodeId) -> NodePath {
    let mut path = NodePath::new();
    let mut current = Some(target);
    while let Some(id) = current {
        path.push(id);
        current = cx.parent(id);
    }
    path.reverse();
    path
}

struct EmitCtx<'a> {
    cx: &'a Cx,
    layout: &'a LayoutEngine,
}

fn emit(ctx: &EmitCtx<'_>, id: NodeId, items: &mut HitItems) {
    let Some(node) = ctx.cx.get(id) else {
        return;
    };
    let rect = ctx.layout.result(id).map(|result| result.rect());

    items.push(HitItem {
        id,
        rect: rect.unwrap_or_else(|| Rect::new(Point::new(0.0, 0.0), 0.0, 0.0)),
        has_rect: rect.is_some(),
        hittable: node.a11y.hittable,
    });

    for child in node.children().iter().copied() {
        emit(ctx, child, items);
    }
}

const fn contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.origin.x
        && point.y >= rect.origin.y
        && point.x < rect.origin.x + rect.width
        && point.y < rect.origin.y + rect.height
}
