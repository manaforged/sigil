use smallvec::SmallVec;

use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, Node, NodeId, TopLayer};
use crate::layout::LayoutResult;
use crate::layout::stack::{Stack, stacked};
use crate::scene::command::Transform3D;

use super::atom::{Backface, SpaceStyle};
use super::theme::fallback_theme;
use super::{StyleAtom, StyleBinding, Transform2D, render_style_for};

impl SpaceStyle {
    pub(crate) fn for_binding(binding: &StyleBinding) -> Self {
        let mut style = Self::FLAT;
        for atom in binding.cascade() {
            if let StyleAtom::Space(atom) = atom {
                style.apply(atom);
            }
        }
        if let Some(overlay) = binding.animator_overlay() {
            style.overlay(overlay);
        }
        style
    }

    fn perspective_at(&self, rect: Rect) -> Option<Transform3D> {
        let distance = self.perspective?;
        let [ox, oy] = self
            .perspective_origin
            .map(|value| f32::from(value) / 10_000.0);
        let origin = Point::new(
            ox.mul_add(rect.width, rect.origin.x),
            oy.mul_add(rect.height, rect.origin.y),
        );
        let flat = Transform3D::IDENTITY;
        Some(flat.around(origin, flat.perspective(distance)))
    }
}

impl Transform2D {
    pub(crate) fn matrix3d(self, rect: Rect, space: &SpaceStyle) -> Transform3D {
        let flat = Transform3D::IDENTITY;
        let [x, y, z] = space.turn;
        let turn = flat
            .translate(0.0, 0.0, space.depth)
            .rotate_x(x)
            .rotate_y(y)
            .rotate_z(z);
        Transform3D::from(self.matrix(rect)).then(flat.around(self.pivot(rect), turn))
    }
}

fn parent_perspective(cx: &Cx, id: NodeId) -> Option<Transform3D> {
    let parent = cx.get(cx.parent(id)?)?;
    let rect = parent.extension::<LayoutResult>()?.rect();
    SpaceStyle::for_binding(parent.extension::<StyleBinding>()?).perspective_at(rect)
}

pub(crate) fn requests_3d(node: &Node) -> bool {
    node.extension::<StyleBinding>()
        .filter(|binding| binding.may_transform())
        .is_some_and(|binding| SpaceStyle::for_binding(binding).preserve)
}

pub(crate) fn preserves(cx: &Cx, id: NodeId) -> bool {
    cx.get(id).is_some_and(|node| {
        requests_3d(node)
            && render_style_for(node, fallback_theme(), None).preserves_3d(
                node.extension::<LayoutResult>()
                    .is_some_and(LayoutResult::clips),
            )
    })
}

pub(crate) fn in_context(cx: &Cx, id: NodeId) -> bool {
    cx.parent(id).is_some_and(|parent| preserves(cx, parent))
        && cx
            .get(id)
            .is_some_and(|node| node.extension::<TopLayer>().is_none())
}

pub(crate) fn projection(
    cx: &Cx,
    id: NodeId,
    (flat, space): (Transform2D, &SpaceStyle),
    rect: Rect,
) -> Option<(Transform3D, bool)> {
    let context = in_context(cx, id);
    if space.is_flat() && !context && !preserves(cx, id) {
        return None;
    }
    let own = flat.matrix3d(rect, space);
    let view = parent_perspective(cx, id).map_or(own, |view| view.then(own));
    let base = cx
        .parent(id)
        .filter(|_| context)
        .and_then(|parent| node_plane(cx, parent));
    let transform = base.map_or(view, |(base, _)| base.then(view));
    Some((transform, space.backface == Backface::Hidden))
}

pub(crate) fn node_plane(cx: &Cx, id: NodeId) -> Option<(Transform3D, bool)> {
    let node = cx.get(id)?;
    let rect = node.extension::<LayoutResult>()?.rect();
    let (flat, space) = node.extension::<StyleBinding>().map_or(
        (Transform2D::IDENTITY, SpaceStyle::FLAT),
        |binding| {
            (
                Transform2D::for_binding(binding),
                SpaceStyle::for_binding(binding),
            )
        },
    );
    projection(cx, id, (flat, &space), rect)
        .or_else(|| (!flat.is_identity()).then(|| (Transform3D::from(flat.matrix(rect)), false)))
}

pub(crate) fn planes(cx: &Cx, id: NodeId) -> Stack {
    let Some(node) = cx.get(id) else {
        return Stack::new();
    };
    let stack = stacked(cx, node).unwrap_or_else(|| SmallVec::from_slice(node.children()));
    if !preserves(cx, id) {
        return stack;
    }
    let mut order: SmallVec<[(f32, NodeId); 8]> = stack
        .iter()
        .map(|&child| (depth(cx, child), child))
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    order.into_iter().map(|(_, child)| child).collect()
}

fn depth(cx: &Cx, id: NodeId) -> f32 {
    let Some(rect) = cx
        .get(id)
        .and_then(|node| node.extension::<LayoutResult>())
        .map(LayoutResult::rect)
    else {
        return 0.0;
    };
    let Some((transform, _)) = node_plane(cx, id) else {
        return 0.0;
    };
    let m = transform.matrix();
    let (x, y) = (
        rect.width.mul_add(0.5, rect.origin.x),
        rect.height.mul_add(0.5, rect.origin.y),
    );
    let along = |row: [f32; 4]| row[0].mul_add(x, row[1].mul_add(y, row[3]));
    let w = along(m[3]);
    if w > 0.0 {
        along(m[2]) / w
    } else {
        f32::NEG_INFINITY
    }
}
