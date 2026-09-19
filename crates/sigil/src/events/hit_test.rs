use crate::hal::{Point, Rect};
use crate::kernel::{Cx, NodeId, TopLayer};
use crate::layout::{LayoutEngine, LayoutResult};
use crate::style::{StyleBinding, Transform2D};
use smallvec::SmallVec;

#[cfg(test)]
#[path = "hit_test_tests.rs"]
mod tests;

pub(crate) type NodePath = SmallVec<[NodeId; 64]>;

type HitItems = SmallVec<[HitItem; 64]>;

#[derive(Debug)]
pub(crate) struct HitTestCache {
    cx_generation: u64,
    layout_generation: u64,
    items: HitItems,
    transforms: SmallVec<[HitTransform; 64]>,
    query: u64,
    transformed: bool,
    pub(super) visual_transforms: bool,
}

#[derive(Clone, Copy, Debug)]
struct HitItem {
    id: NodeId,
    rect: Rect,
    has_rect: bool,
    hittable: bool,
}

#[derive(Debug)]
struct HitTransform {
    parent: Option<usize>,
    transformed: bool,
    query: u64,
    position: Option<Point>,
}

impl Default for HitTestCache {
    fn default() -> Self {
        Self {
            cx_generation: u64::MAX,
            layout_generation: u64::MAX,
            items: HitItems::new(),
            transforms: SmallVec::new(),
            query: 0,
            transformed: false,
            visual_transforms: true,
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
        let hit = self.hit(cx, position)?;
        Some(build_path(cx, hit))
    }

    fn refresh(&mut self, cx: &Cx, layout: &LayoutEngine) {
        let cx_generation = cx.mutation_generation();
        let layout_generation = layout.generation();
        if self.cx_generation == cx_generation && self.layout_generation == layout_generation {
            return;
        }
        self.items.clear();
        self.transforms.clear();
        let ctx = EmitCtx { cx, layout };
        let mut deferred = NodePath::new();
        for root in cx.roots().iter().copied() {
            emit(&ctx, root, None, self, &mut deferred);
        }
        let mut next = 0;
        while let Some(id) = deferred.get(next).copied() {
            next += 1;
            emit(&ctx, id, None, self, &mut deferred);
        }
        self.transformed = self.transforms.iter().any(|item| item.transformed);
        self.cx_generation = cx_generation;
        self.layout_generation = layout_generation;
    }

    pub(super) const fn uses_transforms(&self) -> bool {
        self.transformed && self.visual_transforms
    }

    fn hit(&mut self, cx: &Cx, position: Point) -> Option<NodeId> {
        if !self.uses_transforms() {
            return self.items.iter().rev().find_map(|item| {
                (item.has_rect
                    && item.hittable
                    && contains(item.rect, position)
                    && visible(cx, item.id)
                    && inside_clips(cx, item.id, position))
                .then_some(item.id)
            });
        }
        self.query = self.query.wrapping_add(1);
        if self.query == 0 {
            for item in &mut self.transforms {
                item.query = 0;
            }
            self.query = 1;
        }
        for index in (0..self.items.len()).rev() {
            let item = *self
                .items
                .get(index)
                .expect("hit index belongs to the cache");
            if !item.has_rect || !item.hittable {
                continue;
            }
            let local = self.position_at(cx, index, position);
            if local.is_some_and(|point| contains(item.rect, point))
                && visible(cx, item.id)
                && inside_clips(cx, item.id, position)
            {
                return Some(item.id);
            }
        }
        None
    }

    fn position_at(&mut self, cx: &Cx, target: usize, position: Point) -> Option<Point> {
        let mut chain = SmallVec::<[usize; 64]>::new();
        let mut current = Some(target);
        let mut local = Some(position);
        while let Some(index) = current {
            let item = self
                .transforms
                .get(index)
                .expect("ancestor index belongs to the cache");
            if item.query == self.query {
                local = item.position;
                break;
            }
            chain.push(index);
            current = item.parent;
        }
        for index in chain.into_iter().rev() {
            let transform = self
                .transforms
                .get_mut(index)
                .expect("ancestor index belongs to the cache");
            let item = self.items.get(index).expect("ancestor has a hit item");
            if transform.transformed
                && let Some(binding) = cx
                    .get(item.id)
                    .and_then(|node| node.get_extension::<StyleBinding>())
            {
                local = local
                    .and_then(|point| Transform2D::for_binding(binding).inverse(point, item.rect));
            }
            transform.query = self.query;
            transform.position = local;
        }
        local
    }
}

pub(crate) fn visible(cx: &Cx, target: NodeId) -> bool {
    let mut current = Some(target);
    while let Some(id) = current {
        let Some(node) = cx.get(id) else {
            return false;
        };
        if node
            .get_extension::<crate::style::presence::Presence>()
            .is_some_and(|presence| !presence.interactive())
        {
            return false;
        }
        if node
            .get_extension::<StyleBinding>()
            .is_some_and(StyleBinding::hides)
        {
            return false;
        }
        current = cx.parent(id);
    }
    true
}

fn inside_clips(cx: &Cx, target: NodeId, position: Point) -> bool {
    let mut current = target;
    while let Some(node) = cx.get(current) {
        if node.get_extension::<TopLayer>().is_some() {
            break;
        }
        let Some(parent) = cx.parent(current) else {
            break;
        };
        if let Some(bounds) = cx
            .get(parent)
            .and_then(|ancestor| ancestor.get_extension::<LayoutResult>())
            && bounds.clips()
            && !local_position(cx, parent, position)
                .is_some_and(|point| contains(bounds.rect(), point))
        {
            return false;
        }
        current = parent;
    }
    true
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

fn emit(
    ctx: &EmitCtx<'_>,
    id: NodeId,
    parent: Option<usize>,
    cache: &mut HitTestCache,
    deferred: &mut NodePath,
) {
    let Some(node) = ctx.cx.get(id) else {
        return;
    };
    let rect = ctx.layout.result(id).map(|result| result.rect());

    let index = cache.items.len();
    cache.items.push(HitItem {
        id,
        rect: rect.unwrap_or_else(|| Rect::new(Point::new(0.0, 0.0), 0.0, 0.0)),
        has_rect: rect.is_some(),
        hittable: node.a11y.hittable,
    });
    cache.transforms.push(HitTransform {
        parent,
        transformed: rect.is_some()
            && node
                .get_extension::<StyleBinding>()
                .is_some_and(StyleBinding::may_transform),
        query: 0,
        position: None,
    });

    for child in node.children().iter().copied() {
        if ctx
            .cx
            .get(child)
            .is_some_and(|child_node| child_node.get_extension::<TopLayer>().is_some())
        {
            deferred.push(child);
        } else {
            emit(ctx, child, Some(index), cache, deferred);
        }
    }
}

const fn contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.origin.x
        && point.y >= rect.origin.y
        && point.x < rect.origin.x + rect.width
        && point.y < rect.origin.y + rect.height
}

pub(super) fn local_position(cx: &Cx, target: NodeId, mut point: Point) -> Option<Point> {
    let mut chain = NodePath::new();
    let mut current = Some(target);
    while let Some(id) = current {
        chain.push(id);
        if cx
            .get(id)
            .is_some_and(|node| node.get_extension::<TopLayer>().is_some())
        {
            break;
        }
        current = cx.parent(id);
    }
    for id in chain.into_iter().rev() {
        let node = cx.get(id)?;
        if let Some(binding) = node.get_extension::<StyleBinding>()
            && let Some(result) = node.get_extension::<crate::layout::LayoutResult>()
        {
            point = Transform2D::for_binding(binding).inverse(point, result.rect())?;
        }
    }
    Some(point)
}
